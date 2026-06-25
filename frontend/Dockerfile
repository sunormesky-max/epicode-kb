# Frontend Dockerfile — multi-stage build, nginx runtime.

FROM node:20-slim AS builder

WORKDIR /app

# Copy package files.
COPY package.json ./

# Install dependencies.
RUN npm install

# Copy source.
COPY . .

# Build.
RUN npm run build

# Runtime stage
FROM nginx:alpine

# Use non-root nginx user where possible.
RUN rm -f /etc/nginx/conf.d/default.conf

# Copy built files.
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy nginx config (SPA fallback + API proxy).
RUN echo 'server { \
    listen 5173; \
    server_name localhost; \
    root /usr/share/nginx/html; \
    index index.html; \
    location / { \
        try_files $uri $uri/ /index.html; \
    } \
    location /api { \
        proxy_pass http://backend:3000; \
        proxy_set_header Host $host; \
        proxy_set_header X-Real-IP $remote_addr; \
    } \
}' > /etc/nginx/conf.d/default.conf

EXPOSE 5173

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:5173 || exit 1

CMD ["nginx", "-g", "daemon off;"]
