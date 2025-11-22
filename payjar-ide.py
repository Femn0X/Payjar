#!/usr/bin/env python3
"""
Payjar IDE Launcher
Run this to start the professional Payjar IDE
"""

import sys
from pathlib import Path

# Add the project root to the Python path
project_root = Path(__file__).parent
sys.path.insert(0, str(project_root))

from Payjarnref import createWindow

if __name__ == "__main__":
    # Launch the IDE with default dimensions (1000x1000)
    createWindow(1200, 800)
