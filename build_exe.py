"""
PyInstaller Build Script for Mossambi Desktop Widget.
Compiles a standalone, single-file Windows desktop executable: dist/Mossambi.exe
Usage: python build_exe.py
"""

import os
import subprocess
import sys


def build():
    print("Building Mossambi Desktop Widget...")

    cmd = [
        sys.executable,
        "-m",
        "PyInstaller",
        "--noconfirm",
        "--onedir",
        "--windowed",
        "--name",
        "Mossambi",
        "--add-data",
        "src;src",
        "run_widget.py",
    ]

    print("Running command:", " ".join(cmd))
    res = subprocess.run(cmd)
    if res.returncode == 0:
        print("\nBuild completed successfully!")
        print("Executable located at: dist/Mossambi/Mossambi.exe")
    else:
        print(f"\nBuild failed with exit code {res.returncode}")


if __name__ == "__main__":
    build()
