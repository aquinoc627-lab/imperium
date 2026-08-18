"""Voice Components"""

from pydantic import BaseModel
from typing import List, Dict, Any, Optional

class WhisperSTT:
    def __init__(self):
        pass
    
    async def transcribe(self, audio: bytes) -> str:
        raise NotImplementedError("STT not yet implemented")

class KokoroTTS:
    def __init__(self):
        pass
    
    async def synthesize(self, text: str, voice: str = "default") -> bytes:
        raise NotImplementedError("TTS not yet implemented")

class PorcupineWakeWord:
    def __init__(self):
        pass
    
    async def listen(self) -> bool:
        raise NotImplementedError("Wake word detection not yet implemented")