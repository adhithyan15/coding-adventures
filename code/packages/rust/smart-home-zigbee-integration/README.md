# smart-home-zigbee-integration

`smart-home-zigbee-integration` connects the repository's typed Zigbee
application stack to the normalized D23 smart-home runtime.

The adapter installs a coordinator and ZDO-interviewed Home Automation
endpoints, parses inbound APS data frames and ZCL attribute reports into
normalized state events, and turns authorized runtime light commands into APS
and ZCL wire bytes.

The package deliberately starts above a coordinator transport. A production
coordinator host remains responsible for radio ownership, joining, network-key
handling, APS security, retries, acknowledgements, and delivery of received APS
payloads with their source NWK address.

## Validation

```sh
./smart-home-zigbee-integration/BUILD
cargo clippy -p smart-home-zigbee-integration --all-targets -- -D warnings
```
