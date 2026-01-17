SHELL := /bin/bash

# --- VARIABLES ---------------------------------------------------------------
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

# --- COLORS ------------------------------------------------------------------
GREEN  := \033[0;32m
YELLOW := \033[1;33m
RED    := \033[0;31m
CYAN   := \033[36m
MAGENTA:= \033[35m
NC     := \033[0m

.PHONY: help up rebuild down status logs clean directories docker-up docker-rebuild \
        wait-for-nfs mount unmount warm-up docs docs-rust docs-go docs-clean \
        docker-kill-force

.DEFAULT_GOAL := help

# --- HELP --------------------------------------------------------------------
help:  ## Display this help message
	@awk 'BEGIN {FS = ":.*##"; printf "\n$(MAGENTA)Stripe Slinger Dev Environment$(NC)\nUsage:\n  make $(CYAN)<target>$(NC)\n"} \
	/^[a-zA-Z0-9_-]+:.*?##/ { printf "  $(CYAN)%-20s$(NC) %s\n", $$1, $$2 } \
	/^##@/ { printf "\n$(YELLOW)%s$(NC)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Main Control

up: down directories docker-up wait-for-nfs mount warm-up ## Start full environment (Clean start)
	@echo -e "$(GREEN)[INFO] Environment is fully operational on $(OS)!$(NC)"

rebuild: down directories docker-rebuild wait-for-nfs mount warm-up ## Rebuild images and restart environment
	@echo -e "$(GREEN)[INFO] Environment rebuilt and started!$(NC)"

down: unmount docker-kill-force ## Stop environment and unmount resources (Force)
	@echo -e "$(GREEN)[INFO] Environment stopped.$(NC)"

clean: down ## Wipe data, volumes, and simulated disks (DANGER)
	@echo -e "$(RED)[DANGER] Performing HARD CLEANUP...$(NC)"
	@sudo rm -rf $(STORAGE_DIR)/raid-disks/*
	@sudo rm -rf $(STORAGE_DIR)/raid-data-host/*
	@sudo rm -rf $(STORAGE_DIR)/alloy-data/*
	@echo -e "$(GREEN)[INFO] System is clean.$(NC)"

##@ Monitoring & Logs

status: ## Check NFS mount and Docker container status
	@echo -e "$(YELLOW)--- RAID MOUNT STATUS ---$(NC)"
	@if mountpoint -q $(MOUNT_POINT); then \
		echo -e "Mount: $(GREEN)[OK]$(NC) -> $(MOUNT_POINT)"; \
		ls -F $(MOUNT_POINT); \
	else \
		echo -e "Mount: $(RED)[NOT MOUNTED]$(NC)"; \
	fi
	@echo -e "\n$(YELLOW)--- DOCKER STATUS ---$(NC)"
	@docker compose -f $(COMPOSE_FILE) ps

logs: ## Tail container logs (Follow mode)
	@docker compose -f $(COMPOSE_FILE) logs -f

##@ Documentation

docs: docs-rust docs-go ## Generate and open documentation for all services
	@echo -e "$(GREEN)[INFO] Documentation opened in browser.$(NC)"

docs-rust: ## Generate and open Rust documentation
	@cd $(RUST_DIR) && cargo doc --no-deps --document-private-items
	@$(OPEN_CMD) $(RUST_DIR)/target/doc/raid_simulator/index.html

docs-go: ## Start godoc server and open Metrics Gateway docs
	@if ! pgrep godoc > /dev/null; then \
		nohup godoc -http=:$(DOCS_PORT) > /dev/null 2>&1 & \
		sleep 2; \
	fi
	@$(OPEN_CMD) "http://localhost:$(DOCS_PORT)/pkg/metrics-gateway/internal/simulator/"

docs-clean: ## Kill running godoc server processes
	@pkill godoc || true

##@ Low Level / Internal

mount: ## Mount RAID via NFS (Internal)
	@if ! mountpoint -q $(MOUNT_POINT); then \
		for i in {1..5}; do \
			sudo mount -t nfs $(MOUNT_OPTS) localhost:/ $(MOUNT_POINT) && exit 0; \
			sleep 3; \
		done; \
		exit 1; \
	fi

unmount: ## Unmount RAID directory forcibly (Internal)
	@if mountpoint -q $(MOUNT_POINT); then \
		echo -e "$(YELLOW)[INFO] Unmounting $(MOUNT_POINT)...$(NC)"; \
		sudo umount -f -l $(MOUNT_POINT) 2>/dev/null || true; \
	fi

docker-up: ## Start docker containers (Internal)
	@docker compose -f $(COMPOSE_FILE) up -d

docker-rebuild: ## Rebuild and start docker containers (Internal)
	@docker compose -f $(COMPOSE_FILE) up -d --build

directories: ## Create required storage directories (Internal)
	@mkdir -p $(MOUNT_POINT)
	@mkdir -p $(STORAGE_DIR)/raid-disks
	@mkdir -p $(STORAGE_DIR)/alloy-data
	@chmod 777 $(STORAGE_DIR)/alloy-data 2>/dev/null || true

wait-for-nfs: ## Wait for NFS port availability (Internal)
	@timeout=30; \
	while ! (echo > /dev/null > /dev/tcp/localhost/$(NFS_PORT)) >/dev/null 2>&1; do \
		sleep 1; \
		timeout=$$((timeout - 1)); \
		if [ $$timeout -le 0 ]; then exit 1; fi; \
	done
	@sleep 2

warm-up: ## Initialize RAID controller (Internal)
	@$(TIMEOUT_CMD) 1 bash -c "echo 'init' > $(MOUNT_POINT)/.raidctl" 2>/dev/null || true

docker-kill-force: ## Force kill stuck containers (Internal)
	@echo -e "$(YELLOW)[INFO] Stopping environment...$(NC)"
	@PID=$$(docker inspect --format '{{.State.Pid}}' raid-simulator 2>/dev/null); \
	if [ -n "$$PID" ]; then \
		sudo kill -9 $$PID 2>/dev/null || true; \
	fi
	@if docker rm -f raid-simulator >/dev/null 2>&1; then \
		echo -e " $(GREEN)✔$(NC) Container raid-simulator  $(GREEN)Removed (Force)$(NC)"; \
	fi
	@docker compose -f $(COMPOSE_FILE) down --volumes --remove-orphans 2>/dev/null || true
