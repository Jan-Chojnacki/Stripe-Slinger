#!/bin/bash

MOUNT_POINT="./infra/raid-data-host"
NFS_PORT=2049
CONTAINER_NAME="raid-simulator"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

if ! command -v nc >/dev/null 2>&1; then
    error "netcat (nc) is not installed. Please install it (sudo apt install netcat-openbsd)."
    exit 1
fi

case "$1" in
  up)
    log "Starting Docker containers..."
    mkdir -p "$MOUNT_POINT"
    docker compose up -d

    log "Waiting for NFS port ($NFS_PORT) to be reachable..."
    MAX_RETRIES=30
    COUNT=0
    while ! nc -z localhost $NFS_PORT; do
      sleep 1
      COUNT=$((COUNT + 1))
      if [ $COUNT -ge $MAX_RETRIES ]; then
        error "Timeout: NFS server did not start in time."
        exit 1
      fi
    done

    sleep 2

    log "Mounting RAID on host..."
    if mountpoint -q "$MOUNT_POINT"; then
      warn "Target directory is already mounted."
    else
      sudo mount -t nfs4 -o port=$NFS_PORT,nolock,tcp localhost:/ "$MOUNT_POINT"
      if [ $? -eq 0 ]; then
        log "Success! RAID is now available at: $(realpath $MOUNT_POINT)"
      else
        error "Failed to mount NFS share."
        exit 1
      fi
    fi
    ;;

  down)
    log "Stopping the environment..."

    if mountpoint -q "$MOUNT_POINT"; then
      log "Unmounting the RAID directory from host..."
      sudo umount -l "$MOUNT_POINT"
    else
      warn "RAID directory was not mounted."
    fi

    log "Stopping Docker containers..."
    docker compose down
    log "Done."
    ;;

  status)
    if mountpoint -q "$MOUNT_POINT"; then
      log "RAID MOUNT: [OK]"
      log "Listing files in $MOUNT_POINT:"
      ls -F "$MOUNT_POINT"
    else
      warn "RAID MOUNT: [NOT MOUNTED]"
    fi
    echo "-----------------------------------"
    docker compose ps
    ;;

  *)
    echo "Usage: $0 {up|down|status}"
    exit 1
    ;;
esac
