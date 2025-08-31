import asyncio
import os
from .server import run_server

def main():
    host = os.environ.get("VOICE_HOST", "127.0.0.1")
    port = int(os.environ.get("VOICE_PORT", "7071"))
    asyncio.run(run_server(host, port))

if __name__ == "__main__":
    main()

