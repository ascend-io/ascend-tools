import sys

from ascend_tools.core import Client
from ascend_tools.core import run as run_cli

__all__ = ["Client"]


def main() -> None:
    """CLI entry point."""
    try:
        run_cli(sys.argv)
    except KeyboardInterrupt:
        sys.exit(130)
