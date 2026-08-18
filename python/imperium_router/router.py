"""Router Components"""

from pydantic import BaseModel
from typing import List, Dict, Any, Optional

class ModelInfo(BaseModel):
    id: str
    name: str
    provider: str
    capabilities: List[str]
    context_window: int
    available: bool = True

class ModelRegistry:
    def __init__(self):
        pass
    
    def register(self, model: ModelInfo) -> None:
        pass
    
    def get(self, model_id: str) -> Optional[ModelInfo]:
        return None

class HybridRouter:
    def __init__(self, registry: ModelRegistry):
        self.registry = registry
    
    async def route(self, request: Dict[str, Any]) -> Dict[str, Any]:
        # TODO: Implement routing logic
        raise NotImplementedError("Routing not yet implemented")