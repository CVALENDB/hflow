# hflow

**hflow** is a lightweight, thread-safe task orchestration library for Rust CLI applications. It provides a hierarchical structure to manage and visualize sequential task groups with real-time feedback.

## Features

* **Thread-Safe Execution**: Utilizes atomic-like synchronization using `Arc<Mutex<T>>` for state management across threads.
* **Hierarchical Task Management**: Organizes work into `ExecutionUnit`, `TaskGroup`, and `ProgressManager` for granular control.
* **Real-time Visual Feedback**: Built-in terminal spinner and status indicators with ANSI escape sequences for line clearing.
* **Automatic Error Handling**: Integrated process termination on unit failure to ensure system integrity during critical deployments.

## Architecture

The library follows a three-tier hierarchy:

1. **ProgressManager**: The top-level orchestrator that manages multiple groups.
2. **TaskGroup**: A collection of units that are executed sequentially within the group's context.
3. **ExecutionUnit**: The atomic unit of work that executes a provided closure in a dedicated background thread.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
hflow = { git = "https://github.com/cvalendb/hflow" }
colored = "2.1"

```

## Usage Example

The following example demonstrates how to set up a multi-stage deployment process:

```rust
use hflow::{ProgressManager, TaskGroup, ExecutionUnit, ExecutionStatus};
use std::thread;
use std::time::Duration;

fn main() {
    let mut manager = ProgressManager::new();

    // Group 1: System Initialization
    let mut init_group = TaskGroup::new();
    
    let check_perms = ExecutionUnit::new(
        "Verifying administrator permissions".to_string(),
        |status| {
            thread::sleep(Duration::from_secs(2));
            let mut guard = status.lock().unwrap();
            *guard = ExecutionStatus::Completed;
        }
    );

    init_group.add_unit(check_perms);
    manager.add_group(init_group);

    // Group 2: Network Configuration
    let mut net_group = TaskGroup::new();
    
    let setup_firewall = ExecutionUnit::new(
        "Configuring firewall rules".to_string(),
        |status| {
            thread::sleep(Duration::from_secs(3));
            let mut guard = status.lock().unwrap();
            *guard = ExecutionStatus::Completed;
        }
    );

    net_group.add_unit(setup_firewall);
    manager.add_group(net_group);

    // Start orchestration
    manager.start();
}

```

## Technical Specifications

* **Concurrency Model**: Each `ExecutionUnit` spawns a standard thread for non-blocking logic execution.
* **Terminal UI**: Refresh rate is set to 100ms to balance visual fluidity and CPU overhead.
* **State Management**: Uses `Option::take()` to move closures safely into threads without compromising struct integrity.
? Es ideal para presentar en un portafolio o en el repositorio de **hsupport**. Si necesitas que agregue una sección de "Troubleshooting" o "Contributing", me dices.