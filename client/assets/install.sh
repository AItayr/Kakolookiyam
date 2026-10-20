#!/bin/bash
set -e

echo "=== Installation de Kakolookiyam ==="

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/512x512/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

# 1. Copie de l'executable
cp Kakolookiyam "$BIN_DIR/"
chmod +x "$BIN_DIR/Kakolookiyam"

# 2. Copie de l'icone officielle
cp Kakolookiyam_logo.png "$ICON_DIR/"

# 3. Copie du lanceur .desktop
cp Kakolookiyam.desktop "$APP_DIR/"

# Rafraîchir les caches d'icônes et de menus
update-desktop-database "$APP_DIR" 2>/dev/null || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo "✓ Kakolookiyam est installé avec succès !"
echo "Vous pouvez maintenant le lancer depuis votre menu d'applications."
