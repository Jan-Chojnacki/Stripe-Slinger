SHELL := /bin/bash

REPO_ROOT   := $(shell git rev-parse --show-toplevel)
DEPLOY_DIR  := $(REPO_ROOT)/deploy
STORAGE_DIR := $(REPO_ROOT)/storage
RUST_DIR    := $(REPO_ROOT)/services/raid-simulator
GO_DIR      := $(REPO_ROOT)/services/metrics-gateway

COMPOSE_FILE := $(DEPLOY_DIR)/docker-compose.yml
MOUNT_POINT  := $(STORAGE_DIR)/raid-data-host
NFS_PORT     := 2049
DOCS_PORT    := 6060

OS := $(shell uname -s)
NFS_OPTS_BASE := port=$(NFS_PORT),nolock,tcp,actimeo=0,noac,lookupcache=none,soft,timeo=10,retry=1

ifeq ($(OS),Darwin)
    MOUNT_OPTS  := -o $(NFS_OPTS_BASE),resvport
    TIMEOUT_CMD := perl -e 'alarm shift; exec @ARGV'
    OPEN_CMD    := open
else
    MOUNT_OPTS  := -o $(NFS_OPTS_BASE)
    TIMEOUT_CMD := timeout
    OPEN_CMD    := xdg-open
endif

GREEN  := \033[0;32m
YELLOW := \033[1;33m
RED    := \033[0;31m
CYAN   := \033[36m
NC     := \033[0m

.PHONY: help up rebuild down status logs clean directories docker-up docker-rebuild \
        wait-for-nfs mount unmount warm-up docs docs-rust docs-go docs-clean

.DEFAULT_GOAL := help

help:
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make $(CYAN)<target>$(NC)\n\nTargets:\n"} /^[a-zA-Z0-9_-]+:.*?##/ { printf "  $(CYAN)%-20s$(NC) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

up: directories docker-up wait-for-nfs mount warm-up
	@echo -e "$(GREEN)[INFO] Environment is fully operational on $(OS)!$(NC)"

rebuild: directories docker-rebuild wait-for-nfs mount warm-up
	@echo -e "$(GREEN)[INFO] Environment rebuilt and started!$(NC)"

down: unmount docker-down
	@echo -e "$(GREEN)[INFO] Environment stopped.$(NC)"

docs: docs-rust docs-go
	@echo -e "$(GREEN)[INFO] Dokumentacja otwarta.$(NC)"

docs-rust:
	@cd $(RUST_DIR) && cargo doc --no-deps --document-private-items
	@$(OPEN_CMD) $(RUST_DIR)/target/doc/raid_simulator/index.html

docs-go:
	@if ! pgrep godoc > /dev/null; then \
		nohup godoc -http=:$(DOCS_PORT) > /dev/null 2>&1 & \
		sleep 2; \
	fi
	@$(OPEN_CMD) "http://localhost:$(DOCS_PORT)/pkg/metrics-gateway/internal/simulator/"

docs-clean:
	@pkill godoc || true

status:
	@echo -e "$(YELLOW)--- RAID MOUNT STATUS ---$(NC)"
	@if mountpoint -q $(MOUNT_POINT); then \
		echo -e "Mount: $(GREEN)[OK]$(NC) -> $(MOUNT_POINT)"; \
		ls -F $(MOUNT_POINT); \
	else \
		echo -e "Mount: $(RED)[NOT MOUNTED]$(NC)"; \
	fi
	@echo -e "\n$(YELLOW)--- DOCKER STATUS ---$(NC)"
	@docker compose -f $(COMPOSE_FILE) ps

logs:
	@docker compose -f $(COMPOSE_FILE) logs -f

clean: unmount
	@docker compose -f $(COMPOSE_FILE) down --volumes --remove-orphans
	@sudo rm -rf $(STORAGE_DIR)/raid-disks/*
	@sudo rm -rf $(STORAGE_DIR)/raid-data-host/*
	@sudo rm -rf $(STORAGE_DIR)/alloy-data/*
	@echo -e "$(GREEN)[INFO] System clean.$(NC)"

directories:
	@mkdir -p $(MOUNT_POINT)
	@mkdir -p $(STORAGE_DIR)/raid-disks
	@mkdir -p $(STORAGE_DIR)/alloy-data

docker-up:
	@docker compose -f $(COMPOSE_FILE) up -d

docker-rebuild:
	@docker compose -f $(COMPOSE_FILE) up -d --build

docker-down:
	@docker compose -f $(COMPOSE_FILE) down

wait-for-nfs:
	@timeout=30; \
	while ! (echo > /dev/null > /dev/tcp/localhost/$(NFS_PORT)) >/dev/null 2>&1; do \
		sleep 1; \
		timeout=$$((timeout - 1)); \
		if [ $$timeout -le 0 ]; then \
			exit 1; \
		fi; \
	done
	@sleep 2

mount:
	@if ! mountpoint -q $(MOUNT_POINT); then \
		for i in {1..5}; do \
			sudo mount -t nfs $(MOUNT_OPTS) localhost:/ $(MOUNT_POINT) && exit 0; \
			sleep 3; \
		done; \
		exit 1; \
	fi

unmount:
	@if mountpoint -q $(MOUNT_POINT); then \
		sudo umount -l $(MOUNT_POINT);
