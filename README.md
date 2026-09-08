# Procband

**Procband** is a lightweight terminal UI (TUI) for running and managing
multiple development services from a single command.

Instead of opening a separate terminal for every service:

``` text
npm run gateway
npm run auth
npm run users
npm run orders
```

define them in a `Procfile` and start everything with:

``` bash
Procband
```

Procband starts all services concurrently and gives you a single dashboard
for their status and logs.

## Features

-   Start all services with one command.
-   Read service definitions from a simple `Procfile`.
-   Live TUI showing every service and its current status.
-   View logs for the currently selected service.
-   Capture `stdout` and `stderr` independently.
-   Keep a bounded log history of the most recent 1000 lines per
    service.
-   Navigate between services without leaving the dashboard.
-   Kill a single service without affecting the others.
-   Restart a single service using its original configuration.
-   Clearly distinguish running, exited, crashed, failed, and manually
    killed services.
-   Clean up complete process trees when services are stopped or Procband
    exits.
-   Handles commands that spawn child processes, such as `npm -> node`.

## Why Procband?

Running a multi-service project manually usually means maintaining
several terminal tabs and remembering which command belongs to which
service.

Procband provides a small process supervisor and terminal dashboard in one
application:

``` text
┌─────────────────────────────────────────────────────────────────────┐
│ SERVICES                                                            │
│ ● gateway      RUNNING (pid 1234)                                   │
│ ● auth         RUNNING (pid 1235)                                   │
│ ✖ users        CRASHED (1)                                          │
│ ● orders       RUNNING (pid 1237)                                   │
├─────────────────────────────────────────────────────────────────────┤
│ gateway logs                                                        │
│ Server started on port 4000                                         │
│ Connected to database                                               │
│ Request: GET /health                                                │
│ ...                                                                 │
├─────────────────────────────────────────────────────────────────────┤
│ q: quit   j/k or ↓/↑: switch service   PgUp/PgDn: scroll logs       │
│ x: kill   r: restart                                                │
└─────────────────────────────────────────────────────────────────────┘
```

It is intended as a lightweight, from-scratch alternative to tools such
as Foreman, Overmind, mprocs, and concurrently.

## Requirements

### From source

You need:

-   Rust toolchain
-   Cargo
-   A Unix-like operating system with `sh` and process-group support

The application is designed around Unix process management because it
uses process groups to reliably terminate service process trees.

## Installation

### Build from source

Clone the repository and build the release binary:

``` bash
git clone https://github.com/Dark74A/Procband.git
cd procband

cargo build --release
```

The executable will be available at:

``` text
target/release/procband
```

You can run it directly:

``` bash
./target/release/procband
```

Optionally, install it into Cargo's local binary directory:

``` bash
cargo install --path .
```

Then `Procband` can be run from anywhere:

``` bash
procband
```

## Procfile

Procband uses a simple `name: command` format.

Create a file named `Procfile` in your project directory:

``` text
gateway: npm run gateway
auth: npm run auth
users: npm run users
orders: npm run orders
```

Each line defines one service:

``` text
<service-name>: <command>
```

### Comments and blank lines

Blank lines are ignored, as are lines beginning with `#`:

``` text
# Backend services

gateway: npm run gateway
auth: npm run auth

# Data services
users: npm run users
orders: npm run orders
```

Only the first `:` separates the service name from its command, so
commands containing additional colons are supported:

``` text
order1: npm run start:order:1
```

## Running Procband

Run procband from the directory containing your `Procfile`:

``` bash
procband
```

All configured services are started concurrently.

The dashboard then shows their status and logs.

## Keyboard Controls

| Key        | Action                              |
|------------| ----------------------------------- |
| `q`        | Quit Procband and stop all services |
| `Esc`      | Quit Procband                       |
| `j` / `↓`  | Select next service                 |
| `k` / `↑`  | Select previous service             |
| `PageUp`   | Scroll logs upward                  |
| `PageDown` | Scroll logs downward                |
| `t`        | Kill the selected service           |
| `r`        | Restart the selected service        |

When logs are scrolled upward, new output does not force the view back
to the latest line. Scroll back down to return to the live tail.

## Service Statuses

The dashboard can display the following states:

-   **STARTING** --- the service is being started.
-   **RUNNING** --- the service is currently running.
-   **EXITED** --- the service finished successfully.
-   **CRASHED** --- the service exited with a non-zero status.
-   **FAILED** --- Procband encountered an error while managing the
    process.
-   **KILLED** --- the service was explicitly stopped by the user.

This makes an intentional shutdown distinguishable from an unexpected
crash.

## Process Cleanup

One of Procband's important features is proper process-tree cleanup.

A command such as:

``` bash
npm run start
```

may actually create a process hierarchy similar to:

``` text
procband
  └── sh
       └── npm
            └── node
```

Simply killing the top-level process can leave child processes running
in the background.

Procband places each service in its own process group. When a service is
stopped, the process group is terminated rather than only the direct
child process.

This means commands that spawn additional processes can be shut down
together, preventing orphaned processes from continuing to occupy ports
after Procband exits.

## Logging

Each service has separate `stdout` and `stderr` pipes.

Both streams are read concurrently, allowing a busy service to continue
producing output without one stream blocking the other.

Procband keeps the latest **1000 log lines per service** in memory. Older
lines are automatically discarded, preventing an indefinitely running
service from consuming unlimited memory.

## Project Structure

``` text
Procband/
├── src/
│   ├── main.rs          # Application entry point
│   ├── action.rs        # Keyboard actions and key mapping
│   ├── app.rs           # Main TUI event loop
│   ├── config.rs        # Procfile parsing
│   ├── event.rs         # Events exchanged between components
│   ├── log_buffer.rs    # Bounded per-service log storage
│   ├── manager.rs       # Process creation, output capture and cleanup
│   ├── state.rs         # Application and service state
│   ├── supervisor.rs    # Starting and restarting services
│   ├── tui.rs           # Terminal setup and restoration
│   └── ui.rs            # TUI rendering
├── Procfile             # Example service configuration
├── Cargo.toml           # Rust package and dependency configuration
└── README.md
```

## Architecture Overview

At a high level, Procband works like this:

``` text
                    Procfile
                       │
                       ▼
                  Config Parser
                       │
                       ▼
                Service Supervisor
                  /     |      \
                 /      |       \
             Service  Service  Service
                │        │        │
                └────┬───┴────────┘
                     │
               Event Channel
                     │
                     ▼
                 AppState
                     │
                     ▼
                    TUI
```

Each service runs in its own Tokio task. Process output and lifecycle
changes are sent through an event channel and folded into the
application's central state.

The TUI reads that state and renders the dashboard.

This keeps process management, application state, input handling, and
rendering separated while still allowing everything to run concurrently.

## Development

Run the project in debug mode:

``` bash
cargo run
```

Run tests:

``` bash
cargo test
```


Build an optimized release:

``` bash
cargo build --release
```

Check the project without producing a release binary:

``` bash
cargo check
```

## Limitations

-   The current process-management implementation targets Unix-like
    systems.
-   Services are launched through `sh -c`, so their commands follow
    shell semantics.
-   Log history is intentionally limited to the most recent 1000 lines
    per service.
-   The current UI is terminal-based; there is no graphical interface.

## Contributing

Contributions are welcome.

A typical workflow is:

``` bash
git clone https://github.com/Dark74A/Procband.git
cd Procband

cargo check
cargo test
cargo run
```

Before opening a pull request, make sure the project builds and the test
suite passes.

## License

This project is licensed under the [MIT License](LICENSE).