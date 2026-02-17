# Stripe Slinger | User-Space Distributed Storage Simulation Platform

A high-fidelity, user-space RAID simulation engine orchestrating virtual block devices via FUSE, coupled with a high-throughput telemetry gateway for real-time observability of storage anomalies and recovery algorithms.

## 📺 Demo & Visuals
*Visual documentation of storage geometry and telemetry output.*

### 🛠️ Storage Geometry & CLI Control
*Manual disk orchestration and state verification via terminal interface.*

* **RAID Control & Disk Geometry:**

![RAID CLI Status](/docs/screenshots/RAID_CLI_Status.png)

### 📊 System Health & Performance Monitoring
*High-level visibility into RAID degradation, global state, and resource utilization.*

* **Global State Dashboard (Degraded):**

![System Health Degraded](/docs/screenshots/System_Health_Degraded.png)

* **Telemetry Initialization & State Reset:**

![Telemetry Gap Analysis](/docs/screenshots/Telemetry_Gap_Analysis.png)

![Monitoring State Reset](/docs/screenshots/Monitoring_State_Reset.png)

### 🔍 Technical Deep-Dive & Telemetry
*Granular analysis of physical hardware performance, FUSE abstractions, and RAID logic layers.*

* **Physical Layer Analysis (The "Limping Disk" Detector):**

![Physical Disk Telemetry](/docs/screenshots/Physical_Disk_Telemetry.png)

* **RAID Logic Layer & Engine Efficiency:**

![RAID Logic Deep Dive](/docs/screenshots/RAID_Logic_Deep_Dive.png)

## 🏗️ Architecture & Context
*High-level system design and execution model.*

* **Objective:** Provision of a deterministic environment for benchmarking RAID topologies (0, 1, 3) and validating failure recovery logic without hardware dependency or kernel-level risk.
* **Architecture Pattern:** **Microservices-inspired** decoupled architecture utilizing a sidecar pattern for telemetry. The storage engine follows a Hexagonal (Ports and Adapters) design, isolating core RAID mathematics (`raid-rs`) from the FUSE interface adapter (`raid-cli`).
* **Data Flow:**
    1.  **Ingest:** I/O requests are received via the FUSE mount point and trapped by the kernel.
    2.  **Processing:** Requests are forwarded to the `raid-cli` user-space process and mapped to stripe geometry.
    3.  **Persistence:** Data is striped and persisted to memory-mapped files (`mmap`) representing virtual physical disks.
    4.  **Telemetry:** Operations are asynchronously batched and streamed via gRPC over Unix Domain Sockets (UDS) to a dedicated telemetry **microservice**.

## ⚖️ Design Decisions & Trade-offs
*Technical justifications for architectural and implementation choices.*

* **Interface: fuser (Rust FUSE bindings) over Raw C-FFI**
    * **Context:** Selecting a bridge between the Linux kernel FUSE module and the Rust-based simulation logic.
    * **Decision:** Selection of the `fuser` crate for high-level filesystem abstraction.
    * **Rationale:** `fuser` provides a memory-safe, idiomatic Rust wrapper over the low-level FUSE kernel protocol. It reduces the surface area for memory-related vulnerabilities in the critical I/O path.
    * **Trade-off:** A marginal performance overhead was accepted due to the abstraction layer in exchange for the stability and safety of the storage engine's primary interface.

* **IPC: gRPC over Unix Domain Sockets (UDS)**
    * **Context:** Requirement for high-frequency metric emission (1:1 ratio with I/O operations) between the engine and the gateway.
    * **Decision:** Utilization of gRPC over UDS instead of standard TCP/IP.
    * **Rationale:** Minimization of syscall overhead and latency by bypassing the network stack for strictly local inter-process communication.
    * **Trade-off:** Reduced infrastructure flexibility (local-only communication) was accepted in exchange for significantly lower CPU overhead and latency.

* **Persistence: Memory-Mapped Files (mmap)**
    * **Context:** Simulation of block device access patterns within a user-space environment.
    * **Decision:** Implementation of `mmap` for backing virtual disk storage.
    * **Rationale:** Simplification of striping logic by treating disk storage as byte-addressable memory, reducing the complexity of manual buffer management.
    * **Trade-off:** Direct control over page cache eviction was ceded to the OS kernel, resulting in non-deterministic latency profiles during heavy memory pressure.

## 🧠 Engineering Challenges
*Analysis of non-trivial technical hurdles and implemented solutions.*

* **Challenge: Telemetry Pipeline Saturation & Backpressure**
    * **Problem:** Synchronous metric emission under high IOPS would block the critical storage path, leading to artificial latency inflation and performance degradation.
    * **Implementation:** Development of an asynchronous, non-blocking telemetry pipeline using bounded `tokio::mpsc` channels. A background worker aggregates events into `MetricsBatch` Protobuf messages for streaming.
    * **Outcome:** The telemetry layer operates on a best-effort basis. If the buffer saturates, batches are dropped to preserve the timing fidelity of the primary storage operations.

* **Challenge: Stateless Stripe Geometry Mapping**
    * **Problem:** Mapping linear logical byte offsets to non-contiguous physical offsets across multiple disks (with varying parity placement) typically requires high-overhead lookup tables.
    * **Implementation:** Development of a stateless arithmetic mapper (`retention/volume/mapper.rs`) that calculates `(stripe_index, in_stripe_offset)` and physical coordinates in $O(1)$ time using modular arithmetic and fixed geometry constants.
    * **Outcome:** Achievement of constant-time address translation regardless of volume size, ensuring predictable CPU overhead for random access patterns.

## 🛠️ Tech Stack & Ecosystem
* **Core:** Rust (Simulation Engine), Go (Telemetry Gateway)
* **Persistence:** `mmap`-backed storage, `fuser` crate.
* **CI/CD Infrastructure:** GitLab CI, Custom Docker Runtimes (Go/Rust).
* **Observability:** Prometheus, Grafana Alloy.
* **Tooling:** Protobuf, gRPC, Terraform, Docker Compose.

## 🧪 Quality & Standards
* **CI/CD Pipeline Architecture:**
    * **Modular Orchestration:** Implementation of a granular GitLab CI pipeline with separate job definitions for Linting, Quality Audits, and Testing across both Go and Rust environments.
    * **Environment Determinism:** Utilization of custom-built, hardened Docker runtime images (`golang-runtime`, `rust-runtime`) to ensure total environment parity between local development and CI runners.
    * **Efficiency Gates:** Integration of "No-Code Gates" to optimize runner resources by bypassing the pipeline for non-functional repository changes (e.g., documentation-only updates).
* **Automated Quality Gates:**
    * **Static Analysis:** Enforcement of strict `clippy` (Rust) and `lint` (Go) checks as a prerequisite for any merge request.
    * **Security Auditing:** Automated vulnerability scanning via specialized audit scripts integrated into the pipeline.
    * **Testing Strategy:** Coverage-based verification using dedicated scripts for both unit and integration layers, ensuring the integrity of the storage recovery logic.
* **Engineering Principles:** Strict adherence to Clean Architecture, Infrastructure-as-Code (Terraform), and standardized Merge Request templates to maintain high code-review standards.

## 🙋‍♂️ Authors

**Kamil Fudala**

- [GitHub](https://github.com/FreakyF)
- [LinkedIn](https://www.linkedin.com/in/kamil-fudala/)

**Jan Chojnacki**

- [GitHub](https://github.com/Jan-Chojnacki)
- [LinkedIn](https://www.linkedin.com/in/jan-chojnacki-772b0530a/)

**Jakub Babiarski**

- [GitHub](https://github.com/JakubKross)
- [LinkedIn](https://www.linkedin.com/in/jakub-babiarski-751611304/)

## ⚖️ License

This project is licensed under the [MIT License](LICENSE).
