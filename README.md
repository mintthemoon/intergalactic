# intergalactic
Cosmos/CometBFT secure RPC proxy

## Config
| Setting `ENV` | Default | Description | Options |
| --- | --- | --- | --- |
| backend `IGLTC_BACKEND` | Comet34 | RPC backend type | Comet34 |
| blocked_routes `IGLTC_BLOCKED_ROUTES` | | Blocked routes will not be forwarded to the backend | comma-separated list |
| listen_addr `IGLTC_LISTEN_ADDR` | `127.0.0.1:8080` | Listen address for intergalactic | `<ip>:<port>` |
| rpc_addr `IGLTC_RPC_ADDR` | n/a | RPC backend address | URL (http/https) |
| max_connections `IGLTC_MAX_CONNECTIONS` | 1000 | Max simultaneous connections | int |
| max_subscriptions_per_connection `IGLTC_MAX_SUBSCRIPTIONS_PER_CONNECTION` | 5 | Max websocket subscriptions per connection | int |
| max_request_body_size_bytes `IGLTC_MAX_REQUEST_BODY_SIZE_BYTES` | 1MB | Max size for request body in bytes | int |
| max_response_body_size_bytes `IGLTC_MAX_RESPONSE_BODY_SIZE_BYTES` | 10MB | Max size for response body in bytes | int |
| ws_ping_interval_seconds `IGLTC_WS_PING_INTERVAL_SECONDS` | 30 | Websocket ping interval | int |
