# Table of Contents

1.  [What is pgmer2?](#what-is-pgmer2?)
2.  [How to Launch the Project within Docker](#how-to-launch-the-project-within-docker)
    1.  [Installation in One Command with Docker](#installation-in-one-command-with-docker)
    2.  [Building](#building)
    3.  [Creating a Network](#creating-a-network)
    4.  [Launching Containers](#launching-containers)
3.  [Using psql](#using-psql)
4.  [How to Launch Manually](#how-to-launch-manually)
    1.  [Installation in One Command](#installation-in-one-command)
    2.  [Dependencies](#dependencies)
    3.  [Installing meritrank-service-rust](#installing-meritrank-service-rust)
    4.  [Cloning meritrank-psql-connector](#cloning-meritrank-psql-connector)
    5.  [Using pgrx](#using-pgrx)
    6.  [Testing and Starting](#testing-and-starting)
5.  [Connecting to the Database](#connecting-to-the-database)

<a id="org9cdeabd"></a>

# What is pgmer2?

**pgmer2** is a PostgreSQL Foreign Data Wrapper (FDW) for the [MeritRank service](https://github.com/Intersubjective/meritrank-service-rust).

The **Foreign Data Wrapper (FDW)** is a PostgreSQL extension that enables you to access and manipulate data from external sources as if they were local tables, facilitating seamless integration and real-time querying without the need to move data.

# How to Launch the Project within Docker

## Installation in One Command with Docker
If you prefer not to read through all the steps and copy-paste commands, here’s a single command that prepares everything for you:

```bash
sudo apt install docker git && \
git clone https://github.com/Intersubjective/meritrank-service-rust.git && \
cd meritrank-service-rust && \
docker build -t mr-service . && \
cd .. && \
git clone https://github.com/Intersubjective/meritrank-psql-connector.git && \
cd meritrank-psql-connector && \
docker build -t mr-psql-connector . && \
docker network create my-network && \
docker run --network my-network -p 10234:10234 -e MERITRANK_SERVICE_URL=tcp://0.0.0.0:10234 --detach --name container1 mr-service && \
docker run --network my-network -e POSTGRES_PASSWORD=postgres -e MERITRANK_SERVICE_URL=tcp://container1:10234 --detach --name container2 -p 5432:5432 mr-psql-connector:latest
```

## Building

Before executing any commands, ensure that you have `git` and `docker` installed. We will build both the service and the `psql-connector`.

### Clone the Service Repository and Build It

```bash
git clone https://github.com/Intersubjective/meritrank-service-rust.git
cd meritrank-service-rust/
docker build -t mr-service .
```

### Clone `meritrank-psql-connector` and Build It

```bash
cd .. 
git clone https://github.com/Intersubjective/meritrank-psql-connector.git
cd meritrank-psql-connector/
docker build -t mr-psql-connector .
```

## Creating a Network

A **shared network** is essential for enabling seamless communication between the two Docker containers, `mr-service` and `mr-psql-connector`. By creating a custom network named `my-network`, both containers can resolve each other's names directly, facilitating the connection specified in the `MERITRANK_SERVICE_URL`. This setup allows `mr-psql-connector` to access `mr-service` using its container name (`container1`) as the hostname, eliminating the need for hardcoded IP addresses.

```bash
docker network create my-network
```

## Launching Containers

### Run `mr-service`

Execute the following command to run `mr-service`:

```bash
docker run --network my-network -p 10234:10234 -e MERITRANK_SERVICE_URL=tcp://0.0.0.0:10234 --name container1 mr-service
```

- `--network my-network`: Connects to the shared network created earlier.
- `-p 10234:10234`: Makes the service accessible outside of the container.
- The address `tcp://0.0.0.0` indicates that the service listens on all available network interfaces within the container, allowing connections from any IP address that can reach it on port `10234`.

### Run `mr-psql-connector`

Next, execute this command to run `mr-psql-connector`:

```bash
docker run --network my-network -e POSTGRES_PASSWORD=postgres -e MERITRANK_SERVICE_URL=tcp://container1:10234 --name container2 -p 5432:5432 mr-psql-connector:latest
```

- `--network my-network`: Connects to the shared network created earlier.
- `-e POSTGRES_PASSWORD=postgres`: Sets an environment variable required to establish a password for the default PostgreSQL user; omitting this will result in an error.
- The connection string `MERITRANK_SERVICE_URL=tcp://container1:10234` connects to our `mr-service`, which is mapped to port `10234`.
- `--name container2`: Assigns a specific name to this container.
- `-p 5432:5432`: Makes this service accessible outside of the container.
  
## Using psql

You can now open an interactive shell session as the postgres user inside `container2`, allowing you to manage and interact with your PostgreSQL database directly from within the container.

```bash
docker exec -it container2 su - postgres && psql
```

Here are some basic commands you can use:

```sql
\df
SELECT mr_service_url();
SELECT mr_service();
SELECT mr_create_context('my-context');
```

# How to Launch Manually

## Installation in One Command
If you prefer not to read through all these steps or copy-paste commands, here’s a single command that also checks if Rust is already installed on your system:

```bash
sudo apt-get install build-essential libreadline-dev zlib1g-dev flex bison libxml2-dev libxslt-dev libssl-dev libxml2-utils xsltproc ccache pkg-config rustc clang git cmake && \
if command -v rustc &> /dev/null; then echo "Rust is already installed"; else curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh && source "$HOME/.cargo/env"; fi && \
git clone https://github.com/Intersubjective/meritrank-service-rust.git && \
git clone https://github.com/Intersubjective/meritrank-psql-connector.git
```

### Launching meritrank-service-rust

Next, navigate to the directory and launch `meritrank-service-rust`:

```bash
cd meritrank-service-rust && cargo run > log.txt 2>&1
```

### Go to meritrank-psql-connector, Initialize, and Run pgrx

```bash
cd ../meritrank-psql-connector
cargo install --locked cargo-pgrx
cargo pgrx init
cargo pgrx run
```

## Dependencies

To launch the project, you need to install several dependencies. The installation command for Debian-based distributions is as follows:

### PostgreSQL Build Dependencies:

```bash
sudo apt-get install build-essential libreadline-dev zlib1g-dev flex bison libxml2-dev libxslt-dev libssl-dev libxml2-utils xsltproc ccache pkg-config
```

### Rust Toolchain Dependencies:

```bash
sudo apt-get install rustc
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # Install rustup
source "$HOME/.cargo/env" # Add Rust to PATH
```

### Install Clang Version 7 or Newer and Git:

```bash
sudo apt-get install clang git
```

## Installing meritrank-service-rust

Before working with `meritrank-psql-connector`, you also need to install and launch the service in the background:

```bash
git clone https://github.com/Intersubjective/meritrank-service-rust.git
```

### Directory Structure

Here is how your folder structure should look:

```
intersubjective/
├── meritrank-psql-connector
└── meritrank-service-rust
```

Navigate to the directory and launch `meritrank-service-rust`:

```bash
cd meritrank-service-rust
cargo run
```

## Cloning meritrank-psql-connector

Clone this repository:

```bash
git clone https://github.com/Intersubjective/meritrank-psql-connector.git 
cd meritrank-psql-connector 
```

## Using pgrx

Some of these steps are described on [pgrx’s GitHub page](https://github.com/pgcentralfoundation/pgrx/tree/develop?tab=readme-ov-file):

### Install cargo-pgrx Sub-command:

```bash
cargo install --locked cargo-pgrx 
```

### Initialize PGRX Home at the Root of Your Project:

```bash 
cargo pgrx init 
```

This command downloads all currently supported PostgreSQL versions, compiles them into `${PGRX_HOME}`, and runs `initdb`.

## Testing and Starting

You may want to run automated tests:

```bash 
export RUST_TEST_THREADS=1 
cargo pgrx test 
```

If tests complete without errors, execute:

```bash 
cargo pgrx run 
```

## Connecting to the Database

You can now enter psql and perform actions with MeritRank’s service through the psql connector.

Log in as the postgres user, enter your default password, and execute `psql`:

```bash 
su - postgres 
psql 
```

Here are some commands to ensure that the service is functioning correctly:

```sql 
\df 
SELECT mr_service_url(); 
SELECT mr_service(); 
SELECT mr_create_context('my-context'); 
```
