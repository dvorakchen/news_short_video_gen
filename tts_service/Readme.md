# TTS service

Http server of TTS, only Chinese voice yet

## ENV

```
FLASK_ENV=development | production
FLASK_RUN_HOST=0.0.0.0
FLASK_RUN_PORT=9685
```

## Usage

**Need python 3.10**

```bash
pip install -r requirements.txt
python main.py
```

or docker

```bash
sudo docker build . -t tts_ser
sudo docrun  -p 39685:39685 --gpus all -d tts_ser
```

Then you can send request to 

```

curl --request POST \
  --url 'http://127.0.0.1:39685/tts' \
  --header 'Content-Type: application/json' \
  --data '{
  "texts": [
    "测试 1",
    "测试 2",
    "测试 3"
  ]
}'

```