import sys

from ascend_tools.core import Client, run_cli, run_mcp_http
from ascend_tools.core import __version__ as __version__

__all__ = ["Client", "__version__", "run_cli", "run_mcp_http"]


def main() -> None:
    """CLI entry point."""
    try:
        run_cli(sys.argv)
    except KeyboardInterrupt:
        sys.exit(130)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
