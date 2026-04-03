import asyncio
from typing import Dict, List, Optional
from datetime import datetime
import uuid
from pydantic import BaseModel, Field
from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException
from fastapi.middleware.cors import CORSMiddleware

app = FastAPI()
app.add_middleware(CORSMiddleware, allow_origins=["*"])

class ChatMessage(BaseModel):
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    type: str = "message"
    sender: str
    room: str
    content: str
    timestamp: str = Field(default_factory=lambda: datetime.now().isoformat())

class ReactionMessage(BaseModel):
    type: str = "reaction"
    message_id: str
    user_id: str
    emoji: str
    timestamp: str = Field(default_factory=lambda: datetime.now().isoformat())

class PinMessage(BaseModel):
    type: str = "pin"
    message_id: str
    pinned_by: str
    timestamp: str = Field(default_factory=lambda: datetime.now().isoformat())

class ConnectionManager:
    def __init__(self):
        self.active_connections: Dict[str, Dict[str, WebSocket]] = {}
        self.history: Dict[str, List[ChatMessage]] = {}
        self.pinned_messages: Dict[str, List[str]] = {}  # Room -> List of message_ids
        self.rate_limit: Dict[str, float] = {}

    async def connect(self, websocket: WebSocket, room: str, user: str):
        await websocket.accept()
        if room not in self.active_connections:
            self.active_connections[room] = {}
            self.history[room] = []
            self.pinned_messages[room] = []
        self.active_connections[room][user] = websocket

    def disconnect(self, room: str, user: str):
        if room in self.active_connections and user in self.active_connections[room]:
            del self.active_connections[room][user]

    async def broadcast(self, room: str, message: dict, exclude_user: Optional[str] = None):
        if room in self.active_connections:
            for username, connection in self.active_connections[room].items():
                if username != exclude_user:
                    try:
                        await connection.send_json(message)
                    except Exception:
                        pass

    async def send_to_user(self, websocket: WebSocket, message: dict):
        await websocket.send_json(message)

manager = ConnectionManager()

@app.websocket("/ws/{room}/{user}")
async def websocket_endpoint(websocket: WebSocket, room: str, user: str):
    await manager.connect(websocket, room, user)
    await manager.broadcast(room, {"type": "user_joined", "user": user})
    
    try:
        while True:
            data = await websocket.receive_json()
            now = asyncio.get_event_loop().time()
            if now - manager.rate_limit.get(user, 0) < 0.2:
                continue
            manager.rate_limit[user] = now

            msg_type = data.get("type")
            
            if msg_type == "message":
                chat_msg = ChatMessage(sender=user, room=room, content=data["content"])
                manager.history[room].append(chat_msg)
                await manager.broadcast(room, chat_msg.dict())
            
            elif msg_type == "reaction":
                reaction = ReactionMessage(message_id=data["message_id"], user_id=user, emoji=data["emoji"])
                await manager.broadcast(room, reaction.dict())

            elif msg_type == "pin":
                msg_id = data.get("message_id")
                manager.pinned_messages[room].append(msg_id)
                pin_event = PinMessage(message_id=msg_id, pinned_by=user)
                await manager.broadcast(room, pin_event.dict())

            elif msg_type == "typing":
                await manager.broadcast(room, {"type": "typing", "user": user}, exclude_user=user)

    except WebSocketDisconnect:
        manager.disconnect(room, user)
        await manager.broadcast(room, {"type": "user_left", "user": user})

@app.get("/rooms/{room}/pinned")
async def get_pinned_messages(room: str):
    if room not in manager.pinned_messages:
        raise HTTPException(status_code=404, detail="Room not found")
    return {"pinned": manager.pinned_messages[room]}