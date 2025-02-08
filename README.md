# Fluvio Streaming demo

## Introduction

Demo that produces and consumes messages using the [Fluvio Streaming Engine](https://www.fluvio.io). The message data is serialized and compressed with [bilrost](https://github.com/mumbleskates/bilrost) as it performs really well according to this [benchmark](https://github.com/djkoloski/rust_serialization_benchmark).

## Getting Started

To interact with the Fluvio cluster, install the Fluvio CLI.

Make sure to add $HOME/.fvm/bin and $HOME/.fluvio/bin to your PATH before running these commands:

```shell
curl -fsS https://hub.infinyon.cloud/install/install.sh | bash
fvm install
fluvio cluster set --endpoint 127.0.0.1:9003
```

To create a test topic in Fluvio, run the following command:

```shell
fluvio profile add docker 127.0.0.1:9103 docker
fluvio topic create myio --retention-time '7 days' --segment-size '5 Ki' --max-partition-size '30 Ki'
fluvio topic list
```

Run the solution locally with [bacon](https://github.com/Canop/bacon) by executing `bacon`. Enter 'p' in Bacon to run the producer and enter 'c' to run the consumer. Enter 't' in Bacon to run the test suite with [cargo-nextest](https://nexte.st). 

<img width="1480" alt="image" src="https://github.com/user-attachments/assets/b9a3cd93-7b84-4c61-b085-12afdcf2a1cb" />
