#!/bin/bash
set -e

REPO="A-Fisk/arrrv"
TAG="${1:-latest}"

if [ "$TAG" = "latest" ]; then
  echo "Fetching latest release..."
  TAG=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
  if [ -z "$TAG" ]; then
    echo "Error: Could not determine latest release tag"
    exit 1
  fi
fi

echo "Installing arrrv $TAG..."

# Detect OS and architecture
case "$(uname -s)" in
  Darwin)
    case "$(uname -m)" in
      arm64)
        TARGET="aarch64-apple-darwin"
        ;;
      x86_64)
        TARGET="x86_64-apple-darwin"
        ;;
      *)
        echo "Error: Unsupported macOS architecture: $(uname -m)"
        exit 1
        ;;
    esac
    ;;
  Linux)
    case "$(uname -m)" in
      x86_64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
      *)
        echo "Error: Linux only supports x86_64. Found: $(uname -m)"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Error: Unsupported OS: $(uname -s)"
    exit 1
    ;;
esac

# Determine install directory
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"

# Download and extract
URL="https://github.com/$REPO/releases/download/$TAG/arrrv-$TARGET.tar.gz"
echo "Downloading from: $URL"

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

if ! curl -LsSf "$URL" -o "$TEMP_DIR/arrrv.tar.gz"; then
  echo "Error: Failed to download arrrv. Check your internet connection and that the tag exists."
  exit 1
fi

echo "Extracting..."
tar -xzf "$TEMP_DIR/arrrv.tar.gz" -C "$TEMP_DIR"

# Find the binary (it's in arrrv-$TARGET/bin/arrrv)
BINARY_PATH="$TEMP_DIR/arrrv-$TARGET/bin/arrrv"
if [ ! -f "$BINARY_PATH" ]; then
  echo "Error: Binary not found at expected location: $BINARY_PATH"
  ls -la "$TEMP_DIR/"
  exit 1
fi

cp "$BINARY_PATH" "$INSTALL_DIR/arrrv"
chmod +x "$INSTALL_DIR/arrrv"

echo ""
echo "✓ arrrv $TAG installed successfully!"
echo ""
echo "Binary location: $INSTALL_DIR/arrrv"
echo ""

# Check if install dir is in PATH
if [[ ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
  echo "✓ $INSTALL_DIR is already in your \$PATH"
  echo ""
  echo "You can now run: arrrv --help"
else
  echo "⚠ $INSTALL_DIR is NOT in your \$PATH"
  echo ""
  echo "Add this line to your shell profile (~/.zshrc, ~/.bashrc, etc):"
  echo ""
  echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
  echo ""
  echo "Then restart your terminal or run: source ~/.zshrc"
fi
