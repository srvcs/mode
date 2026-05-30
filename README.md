# srvcs-mode

The statistical-mode service of the srvcs.cloud distributed standard library.

Its single concern: **what is the most frequent value in a list of integers?**
It counts how often each integer appears and reports the most frequent one. On a
tie it returns the **smallest** of the most-frequent values.

`srvcs-mode` is a **leaf**: it depends on no other service and makes no network
calls. All work is local.

```text
result = the integer that appears most often in values
         (ties broken by choosing the smallest such integer)
```

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Report the most frequent integer in `values` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"values": [1, 2, 2, 3]}'
# {"values":[1,2,2,3],"result":2}

curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"values": [4, 4, 5, 5]}'
# {"values":[4,4,5,5],"result":4}
```

Responses:

- `200 {"values": [...], "result": <int>}` — evaluated. `result` is the most
  frequent integer; on a tie it is the smallest of the most-frequent integers.
- `422 {"error": "values must be a non-empty list of integers"}` — the list is
  empty, or some element of `values` is not a JSON integer.

The result is always an `i64`. A singleton list returns its only element; a list
of all-distinct integers (every element ties at count one) returns the smallest.

## Dependencies

None. `srvcs-mode` is a leaf comparison service. Because it owns its own
validation, it rejects an empty list or any non-integer element directly with
`422` rather than forwarding to a dependency.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared
standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
