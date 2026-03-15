import sys

from ascend_tools.core import Client, run_cli, run_mcp_http

__all__ = ["Client", "run_cli", "run_mcp_http"]


def main() -> None:
    """CLI entry point."""
    try:
        run_cli(sys.argv)
    except KeyboardInterrupt:
        sys.exit(130)
    except (RuntimeError, ValueError) as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
