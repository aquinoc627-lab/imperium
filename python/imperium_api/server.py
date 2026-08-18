"""FastAPI Server"""

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

def create_app() -> FastAPI:
    app = FastAPI(
        title="IMPERIUM API",
        version="0.1.0-dev",
        description="The Self-Synthesizing Intent Runtime",
    )
    
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )
    
    @app.get("/health")
    async def health():
        return {"status": "ok", "version": "0.1.0-dev"}
    
    return app

router = None  # TODO: Add API routes