# DCCNET Link Layer Emulator

## Overview
The DCCNET Link Layer Emulator is a project designed to simulate a link layer protocol for a fictitious network named DCCNET. The emulator handles framing, sequencing, error detection, and data retransmission, utilizing TCP for communication between endpoints.

## Features
- **Framing:** Implements a frame structure that includes synchronization patterns, error detection using Internet checksum, payload size, frame ID, and flags for control bits.
- **Sequencing:** Uses a stop-and-wait flow control algorithm with frame IDs restricted to 0 or 1.
- **Error Detection:** Implements checksum validation to detect transmission errors and manage retransmissions.
- **End-of-Transmission:** Utilizes the END flag to indicate the end of data transmission.
- **Unrecoverable Errors:** Handles connection reset using the RST flag.

## Frame Format
A DCCNET frame consists of the following fields:
- **SYNC (32 bits):** Synchronization sequence (0xDCC023C2, repeated twice).
- **Checksum (16 bits):** Internet checksum of the frame.
- **Length (16 bits):** Payload size in bytes (maximum of 4096 bytes).
- **ID (16 bits):** Frame identifier (0 or 1).
- **Flags (8 bits):** Control bits (ACK, END, RST).
- **Data (variable):** Payload data.

## Implementation Details
- The emulator implements the stop-and-wait protocol for flow control, allowing only one frame to be transmitted at a time.
- Transmissions and receptions are managed concurrently to support full-duplex communication.
- Error recovery is achieved by monitoring synchronization patterns and retransmitting corrupted frames.

## Applications
1. **dccnet-md5:** Computes MD5 checksums for received ASCII text lines and transmits them back to the server as hexadecimal strings.

   **Build and Run:**
   ```bash
   cargo build --release --bin dccnet-md5
   ./target/release/dccnet-md5 <IP>:<PORT> <GAS>
   ```

2. **dccnet-xfer:** Transfers files bidirectionally, allowing data exchange between two endpoints.

   - **Build and Run in Server Mode:**
     ```bash
     cargo build --release --bin dccnet-xfer
     ./target/release/dccnet-xfer -s <PORT> <INPUT> <OUTPUT>
     ```
   
   - **Build and Run in Client Mode:**
     ```bash
     cargo build --release --bin dccnet-xfer
     ./target/release/dccnet-xfer -c <IP>:<PORT> <INPUT> <OUTPUT>
     ```
