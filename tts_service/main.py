import torch
import uuid
import os
from io import BytesIO
from TTS.api import TTS
from flask import Flask, request, send_file, jsonify
from pydub import AudioSegment
from dotenv import load_dotenv

load_dotenv()

env = os.getenv('FLASK_ENV', 'development')

app = Flask(__name__)

if env == 'production':
    from config import ProductionConfig
    app.config.from_object(ProductionConfig)
else:
    from config import DevelopmentConfig
    app.config.from_object(DevelopmentConfig)



# Get device
device = "cuda" if torch.cuda.is_available() else "cpu"

# List available 🐸TTS models

tts = TTS("tts_models/zh-CN/baker/tacotron2-DDC-GST")

# tts.tts_to_file(text="2024年7月，四川一女子实名举报前公婆资产近1亿元涉嫌贪腐一事引发关注，当地有关部门介入调查后至今尚未公布结果。", 
#                 file_path="output.wav")


TEMP_DIR = "temp_wavs"
os.makedirs(TEMP_DIR, exist_ok=True)

SILENCE = AudioSegment.silent(duration=0)


def text_to_wav(text, file_path):
    """将文本转为 WAV 文件"""
    
    print(f'handling "{text}"')
    tts.tts_to_file(text=text, 
      file_path=file_path)
    
    return file_path


@app.route('/tts', methods=['POST'])
def generate_tts():
    """
    接收 JSON 数据：
    {
        "texts": ["文本1", "文本2", ...]
    }
    返回拼接后的 WAV 文件
    """
    data = request.get_json()
    texts = data.get('texts', [])


    if not isinstance(texts, list) or not all(isinstance(t, str) for t in texts):
        return jsonify({"error": "Invalid input: 'texts' should be a list of strings."}), 400

    temp_files = []
    combined_audio = AudioSegment.empty()

    try:
        for i, text in enumerate(texts):
            if not text.strip():
                continue
            temp_file = os.path.join(TEMP_DIR, f"{uuid.uuid4()}.wav")
            text_to_wav(text, temp_file)
            segment = AudioSegment.from_wav(temp_file)
            combined_audio += segment
            if i < len(texts) - 1:
                combined_audio += SILENCE
            temp_files.append(temp_file)

        # 导出最终合并的音频
        # wav_name = f"{uuid.uuid4()}.wav"

        output_io = BytesIO()
        combined_audio.export(output_io, format="wav")
        # combined_audio.export(f"./output_wav/{wav_name}", format="wav")

        return send_file(output_io, mimetype='audio/wav', as_attachment=True, download_name="combined_output.wav")

    except Exception as e:
        return jsonify({"error": str(e)}), 500

    finally:
        # clear temp files
        for f in temp_files:
            if os.path.exists(f):
                os.remove(f)


if __name__ == '__main__':
    host = os.getenv('FLASK_RUN_HOST', '0.0.0.0')
    port = os.getenv('FLASK_RUN_PORT', 39685)
    app.run(host, port=port)
