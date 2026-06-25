#!/usr/bin/env bash
# Development script: starts both backend and frontend concurrently.
set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BACKEND_DIR="$PROJECT_ROOT/backend"
FRONTEND_DIR="$PROJECT_ROOT/frontend"

echo "🧠 Starting epicode-kb development servers..."
echo ""

# Start backend
echo "→ Starting backend (Rust/Axum)..."
cd "$BACKEND_DIR"
cargo run &
BACKEND_PID=$!

# Start frontend
echo "→ Starting frontend (Vite/React)..."
cd "$FRONTEND_DIR"
npm run dev &
FRONTEND_PID=$!

echo ""
echo "✅ Development servers started:"
echo "   Backend:  http://localhost:3000"
echo "   Frontend: http://localhost:5173"
echo ""
echo "Press Ctrl+C to stop both servers."

# Trap Ctrl+C to kill both processes
trap 'kill $BACKEND_PID $FRONTEND_PID 2>/dev/null; exit 0' INT TERM

# Wait for either process to exit
wait
