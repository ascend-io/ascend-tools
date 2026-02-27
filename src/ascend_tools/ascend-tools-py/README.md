# ascend-tools-py

[PyO3](https://pyo3.rs) bindings for the [Ascend](https://www.ascend.io) REST API SDK and CLI.

This crate produces the `ascend_tools.core` native Python module via [maturin](https://www.maturin.rs). It exposes the `Client` class (from [`ascend-tools-core`](../ascend-tools-core)) and the `run()` CLI entry point (from [`ascend-tools-cli`](../ascend-tools-cli)) to Python.

## Install

```bash
uv tool install ascend-tools    # CLI
uv pip install ascend-tools     # library
```

## Usage

```python
from ascend_tools import Client

client = Client()  # reads from env vars
client.list_runtimes()
client.run_flow(runtime_uuid="...", flow_name="sales")
```

All methods return `dict` or `list[dict]`. All parameters are keyword-only.

See the [top-level README](../../../README.md) for full documentation.
