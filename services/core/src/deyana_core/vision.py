import base64
import json
import logging
from io import BytesIO
from typing import Optional
from urllib.error import URLError, HTTPError
from urllib.request import Request, urlopen

from PIL import ImageGrab

logger = logging.getLogger(__name__)

class VisionService:
    def __init__(self, ollama_host: str = "http://127.0.0.1:11434", profile: str = "low_spec"):
        self.ollama_host = ollama_host
        self.model = "llava:34b" if profile == "ultra" else "llava:latest"
        self.vision_model = self.model

    def capture_screen_base64(self) -> Optional[str]:
        """Captures the primary screen and returns it as a base64 encoded JPEG string."""
        try:
            # Grab the entire screen
            screenshot = ImageGrab.grab()
            
            # Convert to RGB (in case of RGBA)
            if screenshot.mode != 'RGB':
                screenshot = screenshot.convert('RGB')
                
            # Resize slightly to save memory/VRAM if it's very large, e.g. max 1920x1080
            screenshot.thumbnail((1920, 1080))

            # Save to BytesIO
            buffer = BytesIO()
            screenshot.save(buffer, format="JPEG", quality=85)
            buffer.seek(0)
            
            # Convert to base64
            img_str = base64.b64encode(buffer.read()).decode('utf-8')
            return img_str
        except Exception as e:
            logger.error(f"Failed to capture screen: {e}")
            return None

    def query_screen(self, prompt: str) -> str:
        """Takes a screenshot and sends it to the local vision model."""
        base64_image = self.capture_screen_base64()
        if not base64_image:
            return "Error: Could not capture the screen."

        url = f"{self.ollama_host}/api/chat"
        payload = {
            "model": self.vision_model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt,
                    "images": [base64_image]
                }
            ],
            "stream": False
        }
        
        request = Request(url, data=json.dumps(payload).encode('utf-8'), method="POST")
        request.add_header("Content-Type", "application/json")
        
        try:
            with urlopen(request, timeout=120) as response:
                result = json.loads(response.read().decode('utf-8'))
                if "message" in result and "content" in result["message"]:
                    return result["message"]["content"].strip()
                return "Error: Unexpected response format from vision model."
        except HTTPError as e:
            if e.code == 404:
                return f"Error: Vision model '{self.vision_model}' not found. Please run 'ollama run {self.vision_model}'."
            return f"Error: Vision model returned HTTP {e.code}."
        except URLError as e:
            return f"Error: Could not connect to local Ollama instance: {e.reason}"
        except Exception as e:
            return f"Error: Failed to process vision request: {str(e)}"
