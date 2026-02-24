import sys

from ascend_ops.core import run as _run


def main() -> None:
    """CLI entry point."""
    try:
        _run(sys.argv)
    except KeyboardInterrupt:
        sys.exit(130)
