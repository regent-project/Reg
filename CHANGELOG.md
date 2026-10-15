# Release v0.6.0

- Added `validate` mode: checks that the given inventory and/or expected state files are valid (YAML syntax and deserialization) without connecting to any host

# Release v0.5.1

- Updated to use regent-sdk 0.15.1
- Expected state YAML: `Timeout` is now a required, human-readable duration (e.g. `30s`, `5m`) on every attribute

# Release v0.5.0

- Updated to use regent-sdk 0.15.0

## Installation & Update

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
