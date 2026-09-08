#!/usr/bin/env bash
# download-list installer
# Author: Hadi Cahyadi <cumulus13@gmail.com>

set -e

BINARY="dwn"
INSTALL_DIR="${PREFIX:-/usr/local/bin}"

ESC="\033["
CYAN="${ESC}38;2;77;208;225m"
GREEN="${ESC}38;2;102;187;106m"
YELLOW="${ESC}38;2;255;213;79m"
RED="${ESC}38;2;239;83;80m"
RESET="${ESC}0m"

echo -e "📥 ${CYAN}download-list installer${RESET}"
echo ""

if [ ! -f "$BINARY" ]; then
    echo -e "❌ ${RED}Binary '$BINARY' not found. Run this from the extracted package directory.${RESET}"
    exit 1
fi

if [ ! -w "$INSTALL_DIR" ]; then
    echo -e "⚠️  ${YELLOW}No write permission to $INSTALL_DIR — retrying with sudo…${RESET}"
    sudo cp "$BINARY" "$INSTALL_DIR/$BINARY"
else
    cp "$BINARY" "$INSTALL_DIR/$BINARY"
fi

chmod +x "$INSTALL_DIR/$BINARY"

echo -e "✅ ${GREEN}Installed: $INSTALL_DIR/$BINARY${RESET}"
echo -e "ℹ️  Run ${CYAN}dwn --help${RESET} to get started."
