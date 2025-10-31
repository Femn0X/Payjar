#!/bin/bash
# === GitHub Repo Auto-Cloner ===

# === Configuration ===
REPO_URL="https://github.com/username/repository-name.git"
CLONE_DIR="$HOME/Documents/GitClones"

echo
echo "=== GitHub Repo Cloner ==="
echo

# --- Check if git is installed ---
if ! command -v git &> /dev/null; then
    echo "[ERROR] Git is not installed or not in PATH."
    echo "Please install Git: https://git-scm.com/downloads"
    exit 1
fi

# --- Create target directory if it doesn't exist ---
if [ ! -d "$CLONE_DIR" ]; then
    echo "Creating folder: $CLONE_DIR"
    mkdir -p "$CLONE_DIR"
fi

# --- Change to target directory ---
cd "$CLONE_DIR" || {
    echo "[ERROR] Failed to access $CLONE_DIR"
    exit 1
}

# --- Clone the repository ---
echo "Cloning repository from: $REPO_URL"
git clone "$REPO_URL"

if [ $? -eq 0 ]; then
    echo
    echo "Repository cloned successfully to:"
    echo "$CLONE_DIR"
else
    echo
    echo " Failed to clone repository. Check the URL or your network."
fi

echo
