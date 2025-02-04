# Streaming demo

To interact with the Fluvio cluster, install the Fluvio CLI. 

Make sure to add $HOME/.fvm/bin and $HOME/.fluvio/bin to your PATH.

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
