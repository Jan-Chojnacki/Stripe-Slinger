#!/bin/sh
set -e

SOCKET_DIR="/sockets"
UID_GID="65532:65532"
PERMISSIONS="0770"

echo "[init-sockets] Initializing shared socket directory..."

mkdir -p "$SOCKET_DIR"

echo "[init-sockets] Setting ownership to $UID_GID and mode to $PERMISSIONS"
chown "$UID_GID" "$SOCKET_DIR"
chmod "$PERMISSIONS" "$SOCKET_DIR"

echo "[init-sockets] Done."
