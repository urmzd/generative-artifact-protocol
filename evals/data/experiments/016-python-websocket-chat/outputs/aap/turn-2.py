{
  "protocol": "aap/0.1",
  "id": "artifact-id",
  "version": 1,
  "name": "edit",
  "content": [
    {
      "op": "replace",
      "target": {
        "type": "id",
        "value": "connection-manager"
      },
      "content": "    def __init__(self):\n        self.active_connections: Dict[str, List[WebSocket]] = {}\n        self.history: Dict[str, List[dict]] = {}\n\n    async def connect(self, websocket: WebSocket, room: str):\n        await websocket.accept()\n        if room not in self.active_connections:\n            self.active_connections[room] = []\n        self.active_connections[room].append(websocket)\n\n    def disconnect(self, websocket: WebSocket, room: str):\n        self.active_connections[room].remove(websocket)\n        if not self.active_connections[room]:\n            del self.active_connections[room]\n\n    async def broadcast(self, message: dict, room: str):\n        if room in self.active_connections:\n            for connection in self.active_connections[room]:\n                await connection.send_json(message)\n\n    def add_history(self, room: str, message: dict):\n        if room not in self.history:\n            self.history[room] = []\n        self.history[room].append(message)"
    }
  ]
}