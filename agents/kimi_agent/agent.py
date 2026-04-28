import asyncio
import json
import os
import httpx  # Cleaner async HTTP than requests
import aiofiles

# Constants
MODEL = "kimi-k2.6:cloud"
MASTER_PROMPT_PATH = "/home/jwt/Code/council/agents/system.txt"
KEY_PATH = "/home/jwt/Code/council/agents/ollama.key"

async def get_api_key():
    async with aiofiles.open(KEY_PATH, mode='r') as f:
        content = await f.read()
        return content.strip()

async def api_call(client, key, msg, personal_prompt_path):
    # Read prompts concurrently
    async with aiofiles.open(MASTER_PROMPT_PATH, mode='r') as f:
        master_prompt = await f.read()
    async with aiofiles.open(personal_prompt_path, mode='r') as f:
        personal_prompt = await f.read()

    combined_system = f"{master_prompt}\n\nYOUR SPECIFIC IDENTITY:\n{personal_prompt}"

    payload = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": combined_system},
            {"role": "user", "content": msg}
        ],
        "stream": False # Simpler for this implementation
    }

    headers = {
        "Authorization": f"Bearer {key}",
        "Content-Type": "application/json"
    }

    response = await client.post("https://ollama.com/api/chat", json=payload, headers=headers)
    data = response.json()
    return data.get("message", {}).get("content", "")

async def agent_watcher(agent_config, client, key):
    old_content = ""
    name = agent_config['name']
    in_file = agent_config['in_file']
    out_file = agent_config['out_file']
    personal_prompt = agent_config['personal_prompt']

    # print(f"[*] Started watcher for {name}")

    while True:
        try:
            if os.path.exists(in_file):
                async with aiofiles.open(in_file, mode='r') as f:
                    contents = await f.read()

                # Check for new input and addressing
                if contents != old_content and contents.strip():
                    last_line = contents.strip().split('\n')[-1].lower()
                    
                    # Logic to prevent loops: only reply if named or 'council' mentioned
                    if name.lower() in last_line or "council" in last_line:
                        # print(f"[!] {name} is thinking...")
                        response = await api_call(client, key, contents, personal_prompt)

                        if response.strip():
                            async with aiofiles.open(out_file, mode='a') as f:
                                await f.write(f"{response}\n")
                    
                    old_content = contents
        except Exception as e:
            print(f"[ERROR] {name} watcher: {e}")

        await asyncio.sleep(0.5) # Equivalent to sleep(Duration::from_millis(500))

async def main():
    # Load the council configuration
    with open("config.json", "r") as f:
        config = json.load(f)

    key = await get_api_key()
    
    # Use a single client for connection pooling
    async with httpx.AsyncClient(timeout=60.0) as client:
        tasks = []
        for agent in config['agents']:
            tasks.append(agent_watcher(agent, client, key))
        
        # Run all watchers concurrently
        await asyncio.gather(*tasks)

if __name__ == "__main__":
    asyncio.run(main())