FROM python:3.11-slim

WORKDIR /app

# Install backend dependencies
COPY backend/requirements.txt /app/backend/requirements.txt
RUN pip install --no-cache-dir -r backend/requirements.txt

# Copy backend
COPY backend/ /app/backend/

# Copy data (2D + 3D simulation outputs and visualizations)
COPY data/fusion/ /app/data/fusion/

# Copy paper figures
COPY paper/figures/ /app/paper/figures/

# Copy mission-control dist (pre-built frontend)
COPY mission-control/dist/ /app/mission-control/dist/

EXPOSE 8080

CMD ["python3", "backend/main.py"]
