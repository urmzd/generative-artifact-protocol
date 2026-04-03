<aap:target id="app-code">
import asyncio
from typing import Dict, List, Optional
from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException
from pydantic import BaseModel, Field
from datetime import datetime

app = FastAPI()

class ChatMessage(BaseModel):
    <aap:target id="chat-message-fields">
    type: str = "message"
    room: str
    sender: str
    content: str
    timestamp: str = Field(default_factory=lambda: datetime.utcnow().isoformat())
    </aap:target>

<aap:target id="reaction-message-model">
class ReactionMessage(BaseModel):
    message_id: str
    user_id: str
    emoji: str
    timestamp: str = Field(default_factory=lambda: datetime.utcnow().isoformat())
</aap:target>

class ConnectionManager:
    <aap:target id="connection-manager">
    def __init__(self):
        self.active_connections: Dict[str, List[WebSocket]] = {}
        self.history: Dict[str, List[dict]] = {}

    async def connect(self, websocket: WebSocket, room: str):
        await websocket.accept()
        if room not in self.active_connections:
            self.active_connections[room] = []
        self.active_connections[room].append(websocket)

    def disconnect(self, websocket: WebSocket, room: str):
        self.active_connections[room].remove(websocket)

    async def broadcast(self, message: dict, room: str):
        if room in self.active_connections:
            for connection in self.active_connections[room]:
                await connection.send_json(message)

    def add_history(self, room: str, message: dict):
        if room not in self.history:
            self.history[room] = []
        self.history[room].append(message)
    </aap:target>

manager = ConnectionManager()

@app.websocket("/ws/{room}/{username}")
async def websocket_endpoint(websocket: WebSocket, room: str, username: str):
    <aap:target id="websocket-handler">
    await manager.connect(websocket, room)
    await manager.broadcast({"type": "UserJoined", "user": username}, room)
    
    try:
        while True:
            data = await websocket.receive_json()
            # Basic rate limiting simulation
            await asyncio.sleep(0.1)
            
            msg = ChatMessage(room=room, sender=username, content=data.get("content", ""))
            manager.add_history(room, msg.dict())
            await manager.broadcast(msg.dict(), room)
    except WebSocketDisconnect:
        manager.disconnect(websocket, room)
        await manager.broadcast({"type": "UserLeft", "user": username}, room)
    </aap:target>

@app.get("/rooms")
async def list_rooms():
    <aap:target id="list-rooms-endpoint">
    return {"rooms": list(manager.active_connections.keys())}
    </aap:target>
</aap:target>