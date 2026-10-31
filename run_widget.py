#!/usr/bin/env python3
"""
Launcher for Mossambi Desktop Widget.
Usage: python run_widget.py
"""

import sys
import os

# Ensure project root is in sys.path
root_dir = os.path.dirname(os.path.abspath(__file__))
if root_dir not in sys.path:
    sys.path.insert(0, root_dir)

from src.app import run_app

if __name__ == "__main__":
    run_app()
