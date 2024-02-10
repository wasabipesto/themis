# Load base image and set working directory
FROM node:21 as builder
WORKDIR /app

# Install project dependencies
COPY package*.json ./
RUN npm install

# Copy in the project files
COPY ./index.html ./vite.config.js ./
COPY ./public ./public
COPY ./src ./src

# Build the site
RUN npm run build

# Load nginx image
FROM nginx:1.25 as release
WORKDIR /usr/share/nginx/html

# Remove default nginx static content
RUN rm -rf ./*

# Copy in the nginx configuration
COPY ./nginx/default.conf /etc/nginx/conf.d/default.conf

# Copy the built project from the previous stage
COPY --from=builder /app/dist .

# Expose the port that the application will run on
EXPOSE 80

# Default command to start Nginx and serve the app
CMD ["nginx-debug", "-g", "daemon off;"]