# Examples

Ready-to-adapt files for the three most common automation use cases. Each expected
state is applied to the hosts of an inventory:

| File | Use case | What it does |
|---|---|---|
| [`configuration-management.yaml`](configuration-management.yaml) | Configuration management (server baseline) | base packages, time synchronization, admin group and account, SSH hardening, welcome message from a template, daily cron job |
| [`application-deployment.yaml`](application-deployment.yaml) | Application deployment | container runtime installed and running, web application container of a given version, with its settings and a secret token |
| [`patching.yaml`](patching.yaml) | Patching (system updates) | every package up to date, automatic security updates, disk space kept free for updates |

Inventories:

- [`inventory.yaml`](inventory.yaml): three Debian/Ubuntu servers over SSH (key
  authentication, host keys verified), with variables shared by all hosts
  (`GlobalVars`) and overridden per host (`HostVars`).
- [`localhost.yaml`](localhost.yaml): the machine running reg, to try the examples
  without any server.

The expected states use APT, so they target Debian and Ubuntu; on Fedora/RHEL, use
`!YumDnf` instead of `!Apt` (same fields).

## Running them

```bash
# Check the files (syntax and content) without connecting to anything
reg -i examples/inventory.yaml -s examples/configuration-management.yaml -m validate

# See what is not compliant, without changing anything
reg -i examples/inventory.yaml -s examples/configuration-management.yaml -m assess

# Make the hosts compliant
reg -i examples/inventory.yaml -s examples/configuration-management.yaml -m enforce

# The application's secret token comes from an environment variable
APP_API_TOKEN=... reg -i examples/inventory.yaml -s examples/application-deployment.yaml -m enforce

# Pending updates on every server, then apply them
reg -i examples/inventory.yaml -s examples/patching.yaml -m assess
reg -i examples/inventory.yaml -s examples/patching.yaml -m enforce
```

Before using `inventory.yaml`, set the hosts' addresses (`Endpoint`), the SSH user and
key, and collect the servers' host keys, e.g.
`ssh-keyscan 192.0.2.11 192.0.2.12 192.0.2.13 > ssh/known_hosts`.

## Variables

`{{ name }}` in an expected state is replaced by the host's variable `name` (its
`HostVars`, else the inventory's `GlobalVars`), so one expected state serves hosts
that differ: `srv3` runs another application version and is labelled `staging`.

Some fields are checked when the file is read, before variables are known, so
they take literal values only: e.g. `Hostname`'s name, Docker `Ports`. An inline
template (`!Template`) only sees the variables of its `Context`, whose values can
themselves come from the host's variables (see the welcome message).

## Secrets

Values can come from secrets instead of the files: `SecRef: <name>` reads the
environment variable `<name>` (reg's default provider), `SecRef: <path>` with
`Provider: file` reads a file. Secret values never appear in the live view, the logs
or the results (shown as `********`).
