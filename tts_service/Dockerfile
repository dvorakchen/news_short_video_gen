FROM python:3.10.18-slim

WORKDIR /app

COPY requirements.txt .
RUN apt update
RUN pip install -r requirements.txt

COPY . .

EXPOSE 39685

CMD ["python", "main.py"]