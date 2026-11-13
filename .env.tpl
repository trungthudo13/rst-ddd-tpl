DEBUG=true                           # Enables Swagger UI and OpenAPI
APP_ADDR=127.0.0.1:13000
RUST_LOG="info"
RUST_BACKTRACE=1

HQL_NODE_ID=1
HQL_NODES="1 127.0.0.1:18100 127.0.0.1:18200"
HQL_LISTEN_ADDR_RAFT="127.0.0.1"
HQL_LISTEN_ADDR_API="127.0.0.1"
HQL_DATA_DIR=".data"
HQL_FILENAME_DB="hiqlite.db"
HQL_SECRET_RAFT="raft-secret-change-me"
HQL_SECRET_API="api-secret-change-me"
HQL_LOG_SYNC="immediate_async"          # immediate_async | immediate | interval_<ms>
