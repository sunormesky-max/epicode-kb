# Frontend Dockerfile — multi-stage build
# Builder: node:20, Runtime: nginx:alpine

FROM node:20-slim AS builder

WORKDIR /app

# Copy package files
COPY package.json ./

# Install dependencies
RUN npm install

# Copy source
COPY . .

# Build
RUN npm run build

# Runtime stage
FROM nginx:alpine

WORKDIR /app

# Copy built files
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy nginx config (SPA fallback)
RUN echo 'server { \
    listen 5173; \
    location / { \
        root /usr/share/nginx/html; \
        try_files $uri $uri/ /index.html; \
    } \
    location /api { \
        proxy_pass http://backend:3000; \
        proxy_set_header Host $host; \
    } \
}' > /etc/nginx/conf.d/default.conf

EXPOSE 5173

CMD ["nginx", "-g", "daemon off;"]
