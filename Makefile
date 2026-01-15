SHELL := /bin/bash

REPO_ROOT := $(shell git rev-parse --show-toplevel)
DEPLOY_DIR := $(REPO_ROOT)/deploy
STORAGE_DIR := $(REPO_ROOT)/storage

COMPOSE_FILE := $(DEPLOY_DIR)/docker-compose.yml
MOUNT_POINT := $(STORAGE_DIR)/raid-data-host
NFS_PORT := 2049

OS := $(shell uname -s)

NFS_OPTS_BASE := port=$(NFS_PORT),nolock,tcp,actimeo=0,noac,lookupcache=none,soft,timeo=10,retry=1

ifeq ($(OS),Darwin)
	MOUNT_OPTS := -o $(NFS_OPTS_BASE),resvport
	TIMEOUT_CMD := perl -e 'alarm shift; exec @ARGV'
else
	MOUNT_OPTS := -o $(NFS_OPTS_BASE)
	TIMEOUT_CMD := timeout
endif

GREEN := \033[0;32m
YELLOW := \033[1;33m
RED := \033[0;31m
NC := \033[0m

.PHONY: help up rebuild down status logs clean directories docker-up docker-rebuild wait-for-nfs mount unmount warm-up

.DEFAULT_GOAL := help

help:
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} /^[a-zA-Z0-9_-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

up: directories docker-up wait-for-nfs mount warm-up
	@echo -e "$(GREEN)[INFO] Environment is fully operational on $(OS)!$(NC)"

rebuild: directories docker-rebuild wait-for-nfs mount warm-up #
	@echo -e "$(GREEN)[INFO] Environment rebuilt and started!$(NC)"

down: unmount docker-down
	@echo -e "$(GREEN)[INFO] Environment stopped.$(NC)"

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
	@echo -e "$(RED)[DANGER] Performing HARD CLEANUP...$(NC)"
	@docker compose -f $(COMPOSE_FILE) down --volumes --remove-orphans
	@echo -e "$(RED)[DANGER] Wiping storage directories...$(NC)"
	@sudo rm -rf $(STORAGE_DIR)/raid-disks/*
	@sudo rm -rf $(STORAGE_DIR)/raid-data-host/*
	@sudo rm -rf $(STORAGE_DIR)/alloy-data/*
	@echo -e "$(GREEN)[INFO] System is clean and ready for fresh start.$(NC)"

directories:
	@mkdir -p $(MOUNT_POINT)
	@mkdir -p $(STORAGE_DIR)/raid-disks
	@mkdir -p $(STORAGE_DIR)/alloy-data

docker-up:
	@docker compose -f $(COMPOSE_FILE) up -d

docker-rebuild:
	@echo -e "$(GREEN)[INFO] Rebuilding Docker images...$(NC)"
	@docker compose -f $(COMPOSE_FILE) up -d --build

docker-down:
	@docker compose -f $(COMPOSE_FILE) down

wait-for-nfs:
	@echo -e "$(GREEN)[INFO] Waiting for NFS port ($(NFS_PORT))...$(NC)"
	@timeout=30; \
	while ! (echo > /dev/tcp/localhost/$(NFS_PORT)) >/dev/null 2>&1; do \
		sleep 1; \
		timeout=$$((timeout - 1)); \
		if [ $$timeout -le 0 ]; then \
			echo -e "$(RED)[ERROR] NFS server timeout!$(NC)"; \
			exit 1; \
		fi; \
	done
	@echo -e "$(GREEN)[INFO] Port open. Stabilizing (2s)...$(NC)"
	@sleep 2

mount:
	@if mountpoint -q $(MOUNT_POINT); then \
		echo -e "$(YELLOW)[WARN] Target directory already mounted.$(NC)"; \
	else \
		echo -e "$(GREEN)[INFO] Mounting RAID via NFS on $(OS)...$(NC)"; \
		for i in {1..5}; do \
			sudo mount -t nfs $(MOUNT_OPTS) localhost:/ $(MOUNT_POINT) && exit 0; \
			echo -e "$(YELLOW)[WARN] Mount failed, retrying in 3s...$(NC)"; \
			sleep 3; \
		done; \
		echo -e "$(RED)[ERROR] Failed to mount after 5 attempts.$(NC)"; \
		exit 1; \
	fi

unmount:
	@if mountpoint -q $(MOUNT_POINT); then \
		echo -e "$(GREEN)[INFO] Unmounting RAID...$(NC)"; \
		sudo umount -l $(MOUNT_POINT); \
	else \
		echo -e "$(YELLOW)[WARN] RAID directory was not mounted.$(NC)"; \
	fi

warm-up:
	@echo -e "$(GREEN)[INFO] Warming up RAID controller...$(NC)"
	@$(TIMEOUT_CMD) 1 bash -c "echo 'init' > $(MOUNT_POINT)/.raidctl" 2>/dev/null || true
