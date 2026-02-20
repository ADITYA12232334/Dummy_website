# BLUE WEB COMPANY - Web Application Scanner

A comprehensive web application vulnerability scanner portal with real-time progress tracking. This project features a robust Rust backend and a modern React frontend.

## 🚀 Features

-   **Multi-URL Scanning**: Configure and launch scans for multiple URLs simultaneously.
-   **Real-time Progress**: Track active scans with live updates via Server-Sent Events (SSE).
-   **Vulnerability Dashboard**: View detailed results including severity breakdowns (High, Medium, Low, Info).
-   **Report Management**: Access and download detailed scan reports.
-   **Job Queuing**: Efficient job management using Redis for high performance and reliability.

## 🏗️ Architecture

-   **Frontend**: React (Vite), Tailwind-style CSS, Lucide icons.
-   **Backend**: Rust (Axum), SQLx (SQLite), Redis.
-   **Persistence**: SQLite for configurations and results; Redis for task queuing.

## 🛠️ Getting Started

### Prerequisites

-   **Rust** (latest stable)
-   **Node.js & npm**
-   **Redis** (running on `localhost:6379`)
-   **SQLite3**

### Installation

1.  **Clone the repository**:
    ```bash
    git clone <your-repo-url>
    cd PORTAL_MULTIPLE_URLS
    ```

2.  **Setup Backend**:
    ```bash
    cd backend
    cargo build
    # Ensure your .env (if any) or environment variables are set
    # Default: DATABASE_URL=sqlite:zap_scanner.db, REDIS_URL=redis://127.0.0.1:6379
    cargo run
    ```

3.  **Setup Frontend**:
    ```bash
    cd ../frontend
    npm install
    npm run dev
    ```

4.  **Setup ZAP Worker**:
    ```bash
    cd ../worker
    # Install Python dependencies
    pip install requests redis python-owasp-zap-v24
    
    # Set environment variables (optional)
    # export REDIS_URL=redis://127.0.0.1:6379
    # export ZAP_PROXY=http://127.0.0.1:8080
    
    # Run the worker
    python worker.py
    ```

## 📂 Project Structure

-   `backend/`: Rust source code, database migrations, and job processing logic.
-   `frontend/`: React application, UI components, and API integration.
-   `worker/`: Python scripts for ZAP automation and Redis job processing.
-   `reports/`: (Generated) Directory for scan reports.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

[Specify License, e.g., MIT]
