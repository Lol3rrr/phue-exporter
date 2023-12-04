# Philips Hue Exporter
A Prometheus exporter for Philips Hue Bridge data

## Environment Variables
* `HUE_ADDR`: The raw address of the hue bridge like `192.168.0.10`
* `HUE_USER`: The username for accessing the api (generated using `register` command)

## Setup
### Registering exporter on the bridge
Press the pairing button on the Hue Bridge and register a user name using the `register` command.
This should print out a username that can then be used for all further actions
