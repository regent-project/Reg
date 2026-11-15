//! Live view: prints what each host is doing (attributes, remediations, commands
//! and their output) while the run goes on.

use regent_sdk::hosts::live::{AttributeVerdict, LiveEvent, LiveUpdate, OutputStream};
use regent_sdk::hosts::managed_host::InternalApiCallOutcome;
use std::io::{IsTerminal, Write};
use tokio::sync::mpsc::Receiver;

/// Terminal colours (only when stdout is a terminal and `NO_COLOR` is unset).
#[derive(Clone, Copy)]
pub struct Style {
    colors: bool,
}

impl Style {
    pub fn detect() -> Self {
        Self {
            colors: std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
        }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.colors {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn dim(&self, text: &str) -> String {
        self.paint("2", text)
    }
    fn bold(&self, text: &str) -> String {
        self.paint("1", text)
    }
    fn green(&self, text: &str) -> String {
        self.paint("32", text)
    }
    fn yellow(&self, text: &str) -> String {
        self.paint("33", text)
    }
    fn red(&self, text: &str) -> String {
        self.paint("31", text)
    }
    fn cyan(&self, text: &str) -> String {
        self.paint("36", text)
    }
}

/// Print every update until all senders are gone.
pub async fn print(mut updates: Receiver<LiveUpdate>, style: Style) {
    while let Some(update) = updates.recv().await {
        let line = format_update(&update, style);
        let mut stdout = std::io::stdout().lock();
        // A closed stdout (e.g. `reg ... | head`) must not stop the run
        let _ = writeln!(stdout, "{line}");
    }
}

fn format_update(update: &LiveUpdate, style: Style) -> String {
    let host = style.cyan(&format!("[{}]", update.host_id));
    let text = match &update.event {
        LiveEvent::AttributeStarted { index, name } => {
            style.bold(&format!("==> attribute #{}: {name}", index + 1))
        }
        LiveEvent::AttributeAssessed { verdict, .. } => match verdict {
            AttributeVerdict::Compliant => format!("    {}", style.green("compliant")),
            AttributeVerdict::NonCompliant { remediations } => format!(
                "    {}",
                style.yellow(&format!(
                    "not compliant ({remediations} remediation{})",
                    if *remediations == 1 { "" } else { "s" }
                ))
            ),
            AttributeVerdict::NonCompliantFatal { details } => format!(
                "    {}",
                style.red(&format!("not compliant, cannot be remediated: {details}"))
            ),
            AttributeVerdict::Failed { error } => {
                format!("    {}", style.red(&format!("assessment failed: {error}")))
            }
            _ => format!("    {verdict:?}"),
        },
        LiveEvent::RemediationStarted { remediation, .. } => {
            format!("    -> remediation: {remediation:?}")
        }
        LiveEvent::RemediationFinished { outcome, .. } => match outcome {
            Some(InternalApiCallOutcome::Success(_)) => {
                format!("    {}", style.green("remediation succeeded"))
            }
            Some(InternalApiCallOutcome::AllowedFailure(details)) => format!(
                "    {}",
                style.yellow(&format!("remediation failed (allowed): {details}"))
            ),
            Some(InternalApiCallOutcome::Failure(details)) => {
                format!(
                    "    {}",
                    style.red(&format!("remediation failed: {details}"))
                )
            }
            None => format!("    {}", style.red("remediation could not be applied")),
        },
        LiveEvent::CommandStarted { command, .. } => format!("    {} {command}", style.dim("$")),
        LiveEvent::Output { stream, line, .. } => match stream {
            OutputStream::Stdout => format!("    {} {line}", style.dim("|")),
            OutputStream::Stderr => format!("    {} {}", style.dim("|"), style.yellow(line)),
        },
        LiveEvent::CommandFinished { return_code, .. } => match return_code {
            Some(0) => format!("    {}", style.dim("exit 0")),
            Some(code) => format!("    {}", style.yellow(&format!("exit {code}"))),
            None => format!("    {}", style.red("command could not be run")),
        },
        LiveEvent::Aborted => style.red("aborted"),
        LiveEvent::Dropped { updates } => style.yellow(&format!(
            "... {updates} live update(s) not shown (output too fast)"
        )),
        other => format!("{other:?}"),
    };
    format!("{host} {text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(event: LiveEvent) -> String {
        let update = LiveUpdate {
            run_id: "R1".to_string(),
            host_id: "web1".to_string(),
            job: regent_sdk::Job::Reach,
            seq: 1,
            at: chrono::Utc::now(),
            event,
        };
        format_update(&update, Style { colors: false })
    }

    #[test]
    fn formats_commands_and_output() {
        assert_eq!(
            line(LiveEvent::AttributeStarted {
                index: 0,
                name: "nginx".to_string()
            }),
            "[web1] ==> attribute #1: nginx"
        );
        assert_eq!(
            line(LiveEvent::CommandStarted {
                exec_id: 1,
                command: "apt-get install -y nginx".to_string()
            }),
            "[web1]     $ apt-get install -y nginx"
        );
        assert_eq!(
            line(LiveEvent::Output {
                exec_id: 1,
                stream: OutputStream::Stdout,
                line: "Setting up nginx".to_string()
            }),
            "[web1]     | Setting up nginx"
        );
        assert_eq!(
            line(LiveEvent::CommandFinished {
                exec_id: 1,
                return_code: Some(100)
            }),
            "[web1]     exit 100"
        );
    }
}
