# Reg : a lightweight CLI automation tool

> A lightning-fast, single-binary infrastructure automation and configuration tool built on top of [regent-sdk](https://github.com/regent-project/regent-sdk).



## Overview

**RegRunner** is an all-in-one lightweight 🤖 automation and configuration tool written in 🦀 Rust and powered by [`regent-sdk`](https://github.com/regent-project/regent-sdk). Usage is as simple as possible. Write an inventory file and an expected state file using YAML, and run it using **reg**.


## Installation

For now installation is only available as local build ([Rust required](https://rust-lang.org/tools/install/))

```bash
git clone https://github.com/regent-project/reg
cargo install --path reg
```

## Usage

```bash
# Help
reg -h
# Assess compliance
reg -i /path/to/inventory.yaml -s /path/to/expected_state_description.yaml -m assess 
# Enforce compliance
reg -i /path/to/inventory.yaml -s /path/to/expected_state_description.yaml -m enforce 
# Validate the inventory and/or the expected state description (YAML syntax and deserialization) without doing anything else
reg -i /path/to/inventory.yaml -m validate
reg -i /path/to/inventory.yaml -s /path/to/expected_state_description.yaml -m validate
# Same as assess/enforce, without the live view
reg -i /path/to/inventory.yaml -s /path/to/expected_state_description.yaml -m enforce --no-live
# JSON logs (one flat JSON object per line, live view included) for a log database
reg -i /path/to/inventory.yaml -s /path/to/expected_state_description.yaml -m enforce --log-format json
```

See [`examples/`](examples/) for ready-to-adapt inventories and expected states:
configuration management (server baseline), application deployment and patching.

### Live view

While assessing or enforcing, **reg** shows what each host is doing as it happens: the
attribute being assessed, the remediations being applied, the commands they run and
their output, line by line, while the commands are running:

```
[web1] ==> attribute #1: nginx is installed
[web1]     not compliant (1 remediation)
[web1]     -> remediation: ...
[web1]     $ apt-get install -y nginx
[web1]     | Reading package lists...
[web1]     | Setting up nginx (1.26.0-1) ...
[web1]     exit 0
[web1]     remediation succeeded
```

Commands are shown without the privilege escalation wrapper (the sudo/su password never
appears), and every secret value used during the run is replaced by `********`.
Colours are used when the output is a terminal (disable them with `NO_COLOR=1`). Use
`--no-live` to get only the logs and the final result.

### Logs

The log level is set with `RUST_LOG` (default `info`). Logs never slow a run down: they
are written by a separate thread, and if the output cannot keep up, lines are dropped
and **reg** reports how many at the end.

With `--log-format json`, every line is one flat JSON object, ready for Grafana Loki,
Elasticsearch, etc. The live view then goes into the logs instead of being printed:
each event of a host's run carries `run_id` (one run of one host), `host_id`, `job`
(`assess` / `reach`), `seq` (its position in the run), `event`, the attribute and, for
commands, `exec_id`, `command`, `stream` and `line`. The other logs of the run carry
`run_id` and `host_id` too. To rebuild the live view of a run in Loki (adapt the
stream selector to the labels your log collector sets):

```logql
{service_name="reg"} | json | host_id="web1" | run_id="7F3C0A19B2D4E6F8" | target=~"regent_sdk::live.*"
  | line_format "{{.seq}} {{.event}} {{.command}}{{.line}}"
```

Command output lines are under the target `regent_sdk::live::output`: to keep only the
steps, use `RUST_LOG=info,regent_sdk::live::output=off`.
