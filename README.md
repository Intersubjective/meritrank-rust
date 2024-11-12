# Table Of Contents
- [MeritRank](#meritrank)
- [How to Launch the Project with Docker](#how-to-launch-the-project-with-docker)
  * [Docker Installation](#docker-installation)
  * [One-Command Installation with Docker](#one-command-installation-with-docker)
  * [Building the Project](#building-the-project)
    + [Building the `service`](#building-the--service-)
    + [Building the `psql-connector`](#building-the--psql-connector-)
  * [Creating a Network](#creating-a-network)
  * [Launching the Containers](#launching-the-containers)
    + [Running `mr-service`](#running--mr-service-)
    + [Running `mr-psql-connector`](#running--mr-psql-connector-)
  * [Using psql](#using-psql)
- [Manual Launch Instructions](#manual-launch-instructions)
  * [Quick Installation with One Command](#quick-installation-with-one-command)
    + [Launching the `service`](#launching-the--service-)
    + [Initializing and Running `psql-connector` with pgrx](#initializing-and-running--psql-connector--with-pgrx)
  * [Required Dependencies](#required-dependencies)
    + [PostgreSQL Build Dependencies](#postgresql-build-dependencies)
    + [Rust Toolchain Dependencies](#rust-toolchain-dependencies)
    + [Clang (v7 or Newer) and Git](#clang--v7-or-newer--and-git)
  * [Launching the `service`](#launching-the--service--1)
  * [Launching `psql-connector`](#launching--psql-connector-)
  * [Using pgrx](#using-pgrx)
    + [Installing the cargo-pgrx Sub-command](#installing-the-cargo-pgrx-sub-command)
    + [Initializing PGRX in Your Project](#initializing-pgrx-in-your-project)
  * [Testing and Starting the Service](#testing-and-starting-the-service)
  * [Connecting to the Database](#connecting-to-the-database)

# MeritRank
- [Core](core/README.md)
- [Service](service/README.md)
- [PSQL Connector](psql-connector/README.md)

# How to Launch the Project with Docker
	
## Docker Installation

To install Docker on a Debian-based system, follow the steps provided on [Docker's official site](https://docs.docker.com/engine/install/ubuntu/#installation-methods).

## One-Command Installation with Docker

If you'd like a shortcut that prepares everything in a single command, use the following:

```bash
sudo apt install docker.io git && \
git clone https://github.com/Intersubjective/meritrank-rust.git && \
cd meritrank-rust/service && \
docker build -t mr-service -f ./Dockerfile .. && \
cd ../psql-connector && \
docker build -t mr-psql-connector -f ./Dockerfile .. && \
docker network create my-network && \
docker run --network my-network -p 10234:10234 -e MERITRANK_SERVICE_URL=tcp://0.0.0.0:10234 --detach --name container1 mr-service && \
docker run --network my-network -e POSTGRES_PASSWORD=postgres -e MERITRANK_SERVICE_URL=tcp://container1:10234 --detach --name container2 -p 5432:5432 mr-psql-connector:latest
```

## Building the Project

Ensure you have `git` and `docker` installed before running these commands. We’ll build both the `service` and the `psql-connector`.

### Building the `service`

```bash
git clone https://github.com/Intersubjective/meritrank-rust.git
cd meritrank-rust/service
docker build -t mr-service -f ./Dockerfile ..
```

### Building the `psql-connector`

```bash
cd ../psql-connector
docker build -t mr-psql-connector -f ./Dockerfile ..
```

## Creating a Network

Creating a **shared network** is necessary for smooth communication between `mr-service` and `mr-psql-connector`. By setting up a custom network (`my-network`), the containers can communicate directly by container name, simplifying the connection setup specified in `MERITRANK_SERVICE_URL`.

```bash
docker network create my-network
```

## Launching the Containers

### Running `mr-service`

Use this command to launch `mr-service`:

```bash
docker run --network my-network -p 10234:10234 -e MERITRANK_SERVICE_URL=tcp://0.0.0.0:10234 --name container1 mr-service
```

- `--network my-network`: Connects `mr-service` to the shared network.
- `-p 10234:10234`: Exposes the service on port 10234 for external access.
- The `tcp://0.0.0.0` address configures the service to listen on all network interfaces, enabling connections to port 10234 from any reachable IP.

### Running `mr-psql-connector`

Then, run this command to start `mr-psql-connector`:

```bash
docker run --network my-network -e POSTGRES_PASSWORD=postgres -e MERITRANK_SERVICE_URL=tcp://container1:10234 --name container2 -p 5432:5432 mr-psql-connector:latest
```

- `--network my-network`: Connects `mr-psql-connector` to the shared network.
- `-e POSTGRES_PASSWORD=postgres`: Sets a password for the default PostgreSQL user (required to avoid errors).
- The `MERITRANK_SERVICE_URL=tcp://container1:10234` connection string links to `mr-service` at port 10234.
- `--name container2`: Names the container `container2`.
- `-p 5432:5432`: Exposes PostgreSQL on port 5432 for external access.

## Using psql

You can open an interactive shell session inside `container2` as the PostgreSQL user to manage your database.

```bash
docker exec -it container2 su - postgres && psql
```

Here are some basic commands to try:

```sql
\df
SELECT mr_service_url(); 
SELECT mr_service(); 
SELECT mr_create_context('my-context'); 
```

# Manual Launch Instructions

## Quick Installation with One Command

If you’d prefer to install everything with a single command that also verifies Rust’s installation on your system, use the following:

```bash
sudo apt-get install build-essential libreadline-dev zlib1g-dev flex bison libxml2-dev libxslt-dev libssl-dev libxml2-utils xsltproc ccache pkg-config rustc clang git cmake && \
if command -v rustc &> /dev/null; then echo "Rust is already installed"; else curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh && source "$HOME/.cargo/env"; fi && \
git clone https://github.com/Intersubjective/meritrank-rust.git
```

### Launching the `service`

To start the `service`, navigate to the directory and run:

```bash
cd meritrank-rust/service && cargo run > log.txt 2>&1
```

### Initializing and Running `psql-connector` with pgrx

Proceed to the `psql-connector` directory, initialize, and start `pgrx`:

```bash
cd ../psql-connector
cargo install --locked cargo-pgrx
cargo pgrx init
cargo pgrx run
```

## Required Dependencies

Before launching the project, ensure all necessary dependencies are installed. For Debian-based distributions, use the following commands:

### PostgreSQL Build Dependencies

```bash
sudo apt-get install build-essential libreadline-dev zlib1g-dev flex bison libxml2-dev libxslt-dev libssl-dev libxml2-utils xsltproc ccache pkg-config
```

### Rust Toolchain Dependencies

```bash
sudo apt-get install rustc
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # Install rustup
source "$HOME/.cargo/env" # Add Rust to PATH
```

### Clang (v7 or Newer) and Git

```bash
sudo apt-get install clang git
```

## Launching the `service`

To start `meritrank-service-rust`, use:

```bash
cd meritrank-rust/service
cargo run
```

## Launching `psql-connector`

Navigate to the `psql-connector` directory:

```bash
cd meritrank-rust/psql-connector
```

## Using pgrx

Some steps for setting up `pgrx` are covered on the [pgrx GitHub page](https://github.com/pgcentralfoundation/pgrx/tree/develop?tab=readme-ov-file).

### Installing the cargo-pgrx Sub-command

```bash
cargo install --locked cargo-pgrx 
```

### Initializing PGRX in Your Project

This command will download, compile supported PostgreSQL versions into `${PGRX_HOME}`, and run `initdb`:

```bash
cargo pgrx init
```

## Testing and Starting the Service

To run automated tests:

```bash
export RUST_TEST_THREADS=1 
cargo pgrx test
```

If tests pass without errors, you can launch the service with:

```bash
cargo pgrx run
```

## Connecting to the Database

To access `psql` and interact with MeritRank’s service via the psql connector, log in as the `postgres` user and open `psql`:

```bash
su - postgres
psql
```

You can now execute the following commands to verify that the service is running correctly:

```sql
\df
SELECT mr_service_url();
SELECT mr_service();
SELECT mr_create_context('my-context');
```
