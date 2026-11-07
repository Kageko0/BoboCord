"""Opt-in local WebView2 smoke test. Python standard library only.

Start BoboCord with isolated BOBOCORD_DATA_DIR and WebView2 debug port 9223.
Run `python scripts/desktop_smoke.py flow`, restart, then run `... reopen`.
Never point this test at a database with user data: it creates/deletes fixtures.
The application does not enable remote debugging in normal execution.
"""
import base64
import json
import os
from pathlib import Path
import socket
import struct
import sys
import time
import urllib.parse
import urllib.request


class DevTools:
    def __init__(self, port=9223):
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        deadline = time.monotonic() + 15
        while True:
            try:
                pages = json.load(opener.open(f"http://127.0.0.1:{port}/json", timeout=2))
                page = next(p for p in pages if p.get("type") == "page" and "dioxus" in p.get("url", ""))
                break
            except (OSError, StopIteration):
                if time.monotonic() > deadline:
                    raise RuntimeError("BoboCord WebView2 debug page not available")
                time.sleep(0.1)
        url = urllib.parse.urlparse(page["webSocketDebuggerUrl"])
        self.socket = socket.create_connection(("127.0.0.1", url.port), timeout=30)
        key = base64.b64encode(os.urandom(16)).decode()
        request = f"GET {url.path} HTTP/1.1\r\nHost: 127.0.0.1:{url.port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        self.socket.sendall(request.encode())
        header = b""
        while not header.endswith(b"\r\n\r\n"):
            header += self.socket.recv(1)
        assert b"101" in header.split(b"\r\n", 1)[0], header
        self.sequence = 0

    def read(self, length):
        data = b""
        while len(data) < length:
            block = self.socket.recv(length - len(data))
            if not block:
                raise RuntimeError("WebView2 closed the debugging connection")
            data += block
        return data

    def call(self, method, **params):
        self.sequence += 1
        data = json.dumps({"id": self.sequence, "method": method, "params": params}).encode()
        mask = os.urandom(4)
        size = len(data)
        header = bytes([0x81, 0x80 | size]) if size < 126 else bytes([0x81, 0xFE]) + struct.pack("!H", size)
        self.socket.sendall(header + mask + bytes(v ^ mask[i % 4] for i, v in enumerate(data)))
        fragments = b""
        while True:
            flags, length = self.read(2)
            length &= 0x7F
            if length == 126:
                length = struct.unpack("!H", self.read(2))[0]
            elif length == 127:
                length = struct.unpack("!Q", self.read(8))[0]
            fragments += self.read(length)
            if not flags & 0x80:
                continue
            result = json.loads(fragments)
            fragments = b""
            if result.get("id") == self.sequence:
                if "error" in result:
                    raise RuntimeError(result["error"])
                return result.get("result", {})

    def evaluate(self, expression):
        result = self.call("Runtime.evaluate", expression=expression, awaitPromise=True, returnByValue=True)
        if "exceptionDetails" in result:
            raise RuntimeError(result["exceptionDetails"])
        return result.get("result", {}).get("value")

    def screenshot(self, name):
        result = self.call("Page.captureScreenshot", format="png")
        path = Path("target") / name
        path.write_bytes(base64.b64decode(result["data"]))
        print(f"Screenshot: {path}")


HELPERS = r"""
window.boboTest = {
  assert(value, reason) { if (!value) throw new Error(reason); },
  sleep: ms => new Promise(resolve => setTimeout(resolve, ms)),
  async wait(predicate) { const end=Date.now()+10000; while(!predicate()) { if(Date.now()>end) throw new Error('Timed out: '+predicate); await this.sleep(30); } },
  button(text, parent=document) { const b=[...parent.querySelectorAll('button')].find(b=>b.textContent.trim()===text); this.assert(b,'Button missing: '+text); return b; },
  input(selector,value) { const field=document.querySelector(selector); this.assert(field,'Input missing: '+selector); field.value=value; field.dispatchEvent(new Event('input',{bubbles:true})); },
  async idle() { await this.sleep(80); await this.wait(()=>![...document.querySelectorAll('button')].some(b=>b.textContent.includes('Salvando…')) && !document.querySelector('.feedback')?.textContent.includes('Salvando /')); },
  async modal(op,value) { op.click(); await this.wait(()=>document.querySelector('#edit-dialog')?.open); if(value!==null) { this.input('#item-name',value); await this.sleep(50); } this.button(value===null?'Excluir definitivamente':'Salvar',document.querySelector('dialog')).click(); await this.idle(); },
};
await boboTest.wait(()=>document.querySelector('.add-space') && !document.querySelector('.add-space').disabled);
"""

FLOW = r"""
const t=boboTest;
t.assert(document.querySelectorAll('.space-list button').length===0,'Use an empty isolated test directory');
await t.modal(document.querySelector('.add-space'),'Amigos');
await t.modal(t.button('Renomear',document.querySelector('.sidebar-header')),'Casa');
t.assert(document.querySelector('.sidebar-header h1').textContent==='Casa','Rename space');
await t.modal(document.querySelector('.section-heading button'),'geral');
await t.modal(t.button('Renomear',document.querySelector('.chat-header')),'conversa');
t.input('#message-input','  '); await t.sleep(50); t.button('Enviar ↑').click(); await t.idle();
t.assert(document.querySelector('.feedback[data-error=true]'),'Validation feedback');
t.assert(document.querySelector('#message-input').value==='  ','Keep invalid draft');
t.input('#message-input','Olá, local! <script>window.bad=true</script>'); await t.sleep(50); t.button('Enviar ↑').click(); await t.idle();
t.assert(document.querySelectorAll('.message-row').length===1,'Message rendered');
t.assert(document.querySelector('.message-body').textContent.includes('<script>'),'Message text escaped');
t.assert(!window.bad,'No script injection');
t.assert(document.querySelector('time').textContent.includes('UTC'),'Timestamp');
await t.modal(document.querySelector('.section-heading button'),'segundo');
t.assert(document.querySelectorAll('.message-row').length===0,'Channel isolation');
t.input('#message-input','rascunho segundo'); await t.sleep(50);
t.button('#conversa',document.querySelector('.channel-list')).click(); await t.idle();
t.assert(document.querySelectorAll('.message-row').length===1,'Navigate to first channel');
t.button('#segundo',document.querySelector('.channel-list')).click(); await t.idle();
t.assert(document.querySelector('#message-input').value==='rascunho segundo','Draft per channel');
document.querySelector('[aria-label=Configurações]').click(); await t.sleep(80);
t.button('Claro').click(); await t.idle();
t.assert(document.querySelector('.app-shell').dataset.theme==='light','Light theme');
t.input('#profile-name','Felipe'); await t.sleep(50); t.button('Salvar perfil').click(); await t.idle();
t.assert(document.querySelector('.profile-copy strong').textContent==='Felipe','Profile');
return {crud:true,validation:true,escaped_text:true,drafts:true,theme:true,profile:true};
"""

REOPEN = r"""
const t=boboTest;
t.assert(document.querySelector('.app-shell').dataset.theme==='light','Theme after restart');
t.assert(document.querySelector('.profile-copy strong').textContent==='Felipe','Profile after restart');
t.assert(document.querySelector('.sidebar-header h1').textContent==='Casa','Space after restart');
t.button('#conversa',document.querySelector('.channel-list')).click(); await t.idle();
t.assert(document.querySelectorAll('.message-row').length===1,'Message after restart');
t.button('Excluir',document.querySelector('.chat-header')).click(); await t.wait(()=>document.querySelector('dialog')?.open);
t.assert(document.querySelector('.dialog-actions button').textContent==='Cancelar','Destructive dialog cancel first');
t.button('Cancelar',document.querySelector('dialog')).click(); await t.sleep(80);
t.assert(document.querySelectorAll('.message-row').length===1,'Cancel does not delete');
t.button('#segundo',document.querySelector('.channel-list')).click(); await t.idle();
await t.modal(t.button('Excluir',document.querySelector('.chat-header')),null);
t.assert(document.querySelectorAll('.channel-button').length===1,'Delete channel');
t.assert(document.querySelectorAll('.message-row').length===1,'Other history survives');
await t.modal(t.button('Excluir',document.querySelector('.sidebar-header')),null);
t.assert(document.querySelectorAll('.space-list button').length===0,'Delete space');
t.assert(document.querySelectorAll('.channel-button').length===0,'Cascade channels');
t.assert(document.querySelectorAll('.message-row').length===0,'Cascade messages');
return {restart:true,cancel:true,delete_channel:true,delete_space:true};
"""


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "flow"
    if mode not in ("flow", "reopen", "keyboard"):
        raise SystemExit("Usage: desktop_smoke.py flow|reopen|keyboard")
    tools = DevTools(int(os.environ.get("BOBOCORD_TEST_DEBUG_PORT", "9223")))
    tools.evaluate(f"(async()=>{{{HELPERS}}})()")
    if mode == "keyboard":
        tools.evaluate("""(async()=>{
            document.querySelector('.add-space').click();
            await boboTest.wait(()=>document.querySelector('dialog')?.open);
            boboTest.assert(document.activeElement.id==='item-name','Modal input autofocus');
        })()""")
        for _ in range(7):
            tools.call("Input.dispatchKeyEvent", type="keyDown", key="Tab", code="Tab", windowsVirtualKeyCode=9)
            tools.call("Input.dispatchKeyEvent", type="keyUp", key="Tab", code="Tab", windowsVirtualKeyCode=9)
            time.sleep(0.03)
            assert tools.evaluate("document.querySelector('dialog').contains(document.activeElement)"), "Modal focus trap"
        tools.call("Input.dispatchKeyEvent", type="keyDown", key="Escape", code="Escape", windowsVirtualKeyCode=27)
        tools.evaluate("(async()=>{await boboTest.wait(()=>!document.querySelector('dialog'));})()")
        tools.evaluate("""(async()=>{
            const t=boboTest;
            await t.modal(document.querySelector('.add-space'),'Teclado');
            await t.modal(document.querySelector('.section-heading button'),'texto');
            t.input('#message-input','Linha 1');
            document.querySelector('#message-input').focus();
            await t.sleep(80);
        })()""")
        tools.call("Input.dispatchKeyEvent", type="keyDown", key="Enter", code="Enter", windowsVirtualKeyCode=13, modifiers=8, text="\r")
        tools.call("Input.insertText", text="Linha 2")
        tools.evaluate("""(async()=>{
            await boboTest.sleep(100);
            boboTest.assert(document.querySelectorAll('.message-row').length===0,'Shift+Enter does not send');
            boboTest.assert(document.querySelector('#message-input').value.includes('\\n'),'Shift+Enter inserts newline');
        })()""")
        tools.call("Input.dispatchKeyEvent", type="keyDown", key="Enter", code="Enter", windowsVirtualKeyCode=13, modifiers=0, text="\r")
        tools.evaluate("""(async()=>{
            await boboTest.wait(()=>document.querySelectorAll('.message-row').length===1);
            boboTest.assert(document.querySelector('#message-input').value==='','Enter sends and clears draft');
        })()""")
        tools.call("Emulation.setEmulatedMedia", features=[{"name": "prefers-reduced-motion", "value": "reduce"}])
        tools.evaluate("""(async()=>{
            const t=boboTest;
            t.button('Excluir',document.querySelector('.sidebar-header')).click();
            await t.wait(()=>document.querySelector('dialog')?.open);
            t.assert(getComputedStyle(document.querySelector('dialog')).animationName==='none','Reduced motion');
            t.button('Excluir definitivamente',document.querySelector('dialog')).click(); await t.idle();
        })()""")
        result = {"autofocus": True, "focus_trap": True, "escape": True, "enter": True, "shift_enter": True, "reduced_motion": True}
    else:
        result = tools.evaluate(f"(async()=>{{{FLOW if mode == 'flow' else REOPEN}}})()")
    print(json.dumps(result, ensure_ascii=False))
    for scale in (1, 1.25, 1.5):
        tools.call("Emulation.setDeviceMetricsOverride", width=1000, height=680, deviceScaleFactor=scale, mobile=False)
        assert tools.evaluate("document.querySelector('.app-shell').scrollWidth <= innerWidth"), "Horizontal overflow"
    tools.call("Emulation.setDeviceMetricsOverride", width=740, height=480, deviceScaleFactor=1, mobile=False)
    assert tools.evaluate("document.querySelector('.app-shell').scrollWidth <= innerWidth"), "Resize overflow"
    tools.call("Emulation.clearDeviceMetricsOverride")
    tools.screenshot(f"dioxus-{mode}.png")
    resources = tools.evaluate("performance.getEntriesByType('resource').map(x=>x.name)")
    assert all("dioxus.index.html" in url or url.startswith(("dioxus:", "data:", "about:")) for url in resources), resources
    print("PASS: native WebView2 DOM flow, resize, simulated scales, local resources")


if __name__ == "__main__":
    main()
