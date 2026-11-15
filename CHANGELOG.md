# Release v0.10.0

## What's Changed

### Live view (on by default)

- While assessing or enforcing, reg shows what each host is doing as it happens:
  the attribute being assessed, its verdict, the remediations being applied, the
  commands they run and their output line by line, while the commands are running.
- Commands are shown without the privilege escalation wrapper (the sudo/su password
  never appears), and every secret value used during the run is replaced by
  `********`.
- Colours when the output is a terminal (`NO_COLOR=1` disables them).
- `--no-live` turns the live view off (logs and final result only).

### Logs

- `--log-format text|json` (default `text`). `json` writes one flat JSON object per
  line (`timestamp`, `level`, `target`, `message`, `service`, `version`, then the
  fields of the event and of its spans), ready for a log database such as Grafana
  Loki or Elasticsearch. The live view is then part of the logs: each line carries
  `run_id`, `host_id`, `job` and `seq`, so the run of one host can be rebuilt from
  the database (filter on `run_id`, sort on `seq`).
- Log level set with `RUST_LOG` (default `info`), e.g. `RUST_LOG=debug`.
- Logs never slow reg down: they are written by a dedicated thread; if the output
  cannot keep up, lines are dropped and their count is reported at exit.

### Examples

- New `examples/` for the most common use cases, ready to adapt:
  `configuration-management.yaml` (server baseline), `application-deployment.yaml`
  (containerized web application) and `patching.yaml` (system updates), with an
  SSH inventory (`inventory.yaml`) and a local one (`localhost.yaml`) to try them.
  See `examples/README.md`.

### Other changes

- Updated to regent-sdk 0.24.1 (from 0.17.0): newer attributes, SSH host key
  verification (`HostKey` in the connection method), host lock, SSHOQ client
  `sshoq`.
- An aborted host run is reported as `Aborted`.

### Debian
Download the `.deb` package and install it using:
```bash
sudo dpkg -i ./reg_<version>_amd64.deb
```

### Red Hat
Download the `.rpm` package and install it using:
```bash
sudo rpm -ivh ./reg-<version>.x86_64.rpm
```
