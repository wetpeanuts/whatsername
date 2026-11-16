# whatsername

[![Version](https://img.shields.io/badge/version-0.1.0-blue)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![CI](https://github.com/wetpeanuts/whatsername/actions/workflows/ci.yml/badge.svg)](https://github.com/wetpeanuts/whatsername/actions/workflows/ci.yml)

`whatsername` is a terminal UI that turns a natural language request into a shell command. Describe what you want to do, let an AI agent suggest a command, edit it if needed, and run it in your shell.

## Installation

Install the binary from this repository:

```sh
just install
```

Make sure Cargo's default install directory (`$HOME/.cargo/bin`) is in your `PATH`, then run:

```sh
whatsername
```

## Usage

* Run `whatsername`.
* Type a prompt describing the desired shell command and press `Enter`.
* The suggested command opens in an editable box:
  edit it if needed, press `Enter` to run it, or `Esc`/`Ctrl-C` to cancel.

## Configuration

The agent is configured through the `WHATSERNAME_AGENT` environment variable:

```sh
WHATSERNAME_AGENT="opencode run --print-logs=false" whatsername
```

It defaults to `opencode run --print-logs=false`. The agent is expected to return the suggested command and nothing else, wrapped in a triple-backtick block. The command is executed with the shell specified by `$SHELL`.

## TODO

- [ ] Support optional multiple command suggestions from the agent
- [ ] Deterministic error handling from the agent: prompt the agent to return the command and error in a defined format, and validate the agent output against this format
- [ ] Improve view layout
- [ ] Optionally add styling
- [ ] Print accepted command into stdout prior to running
- [ ] Save command to shell history
