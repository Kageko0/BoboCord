"""F1 native regression tests against an isolated fixture from seed_load.py.
Uses only the standard library and the local WebView2 debug endpoint.
"""
import ctypes
from ctypes import wintypes
import json
import os
import sys
import time
from desktop_smoke import DevTools, HELPERS


def close_window(process_id):
    user32 = ctypes.windll.user32
    found = []
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)

    def visit(handle, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(handle, ctypes.byref(owner))
        title = ctypes.create_unicode_buffer(512)
        user32.GetWindowTextW(handle, title, 512)
        if owner.value == process_id and "BoboCord" in title.value:
            user32.PostMessageW(handle, 0x10, 0, 0)
            found.append(handle)
        return True

    user32.EnumWindows(callback_type(visit), 0)
    assert found, "No owned BoboCord test window found"


SCROLL = r"""
const t=boboTest, list=document.querySelector('#message-history');
await t.sleep(100);
t.assert(document.querySelectorAll('.message-row').length===50,'Only first SQL page mounted');
const atEnd=()=>list.scrollHeight-list.clientHeight-list.scrollTop<74;
t.assert(atEnd(),'Initial page scrolls to end');
const anchor=()=>{
  const top=list.getBoundingClientRect().top;
  const row=[...list.querySelectorAll('.message-row')].find(r=>r.getBoundingClientRect().bottom>top+2);
  return {id:row.dataset.messageId,offset:row.getBoundingClientRect().top-top};
};
const assertAnchor=(previous)=>{
  const row=[...list.querySelectorAll('.message-row')].find(r=>r.dataset.messageId===previous.id);
  t.assert(row,'Visible anchor still mounted');
  const delta=Math.abs(row.getBoundingClientRect().top-list.getBoundingClientRect().top-previous.offset);
  t.assert(delta<2,'Anchor drift: '+delta);
  return delta;
};
list.scrollTop=list.scrollHeight/3; await t.sleep(60);
const previous=anchor();
t.button('Mostrar anteriores',list).click(); await t.idle(); await t.sleep(60);
t.assert(list.querySelectorAll('.message-row').length===100,'Second SQL page');
const prependDrift=assertAnchor(previous);
t.assert(!atEnd(),'Older page must not jump to end');
t.input('#message-input','F1 nova sem salto'); await t.sleep(50); t.button('Enviar ↑').click(); await t.idle(); await t.sleep(60);
const appendDrift=assertAnchor(previous);
t.assert(!atEnd(),'New message must not hijack reader');
list.scrollTop=list.scrollHeight; await t.sleep(60);
t.input('#message-input','F1 acompanha fim'); await t.sleep(50); t.button('Enviar ↑').click(); await t.idle(); await t.sleep(60);
t.assert(atEnd(),'Follow message only when already near end');
for(let i=0;i<8;i++) {
  list.scrollTop=0; await t.sleep(35); const before=anchor();
  t.button('Mostrar anteriores',list).click(); await t.idle(); await t.sleep(50);
  assertAnchor(before);
  t.assert(list.querySelectorAll('.message-row').length<=300,'Bounded DOM under load');
}
t.button('Voltar às recentes',list).click(); await t.idle(); await t.sleep(60);
t.assert(list.querySelectorAll('.message-row').length===50,'Return to bounded recent page');
t.assert(atEnd(),'Explicit recent navigation scrolls to end');
const ids=[...list.querySelectorAll('.message-row')].map(r=>r.dataset.messageId);
t.assert(new Set(ids).size===ids.length,'No duplicate IDs');
window.f1StableRows=[...list.querySelectorAll('.message-row')];
for(const value of ['a','ab','abc']) { t.input('#message-input',value); await t.sleep(25); }
await t.sleep(350);
t.assert(window.f1StableRows.every((r,i)=>r===list.querySelectorAll('.message-row')[i]),'Typing preserves DOM row identity');
return {rows:ids.length,prepend_drift_px:prependDrift,append_drift_px:appendDrift,bounded_dom:true,stable_typing:true};
"""

WRITE = r"""
const t=boboTest;
t.input('#message-input','rascunho persistido do canal de carga'); await t.sleep(50);
t.button('#quiet-1',document.querySelector('.channel-list')).click(); await t.idle();
t.input('#message-input','última edição antes de fechar\n🙂');
await t.sleep(30); // Less than the 250 ms debounce; WM_CLOSE immediately follows.
return {selected:document.querySelector('.channel-button[aria-pressed=true]').textContent};
"""

READ = r"""
const t=boboTest;
t.assert(document.querySelector('.channel-button[aria-pressed=true]').textContent==='#quiet-1','Restore selected channel');
t.assert(document.querySelector('#message-input').value==='última edição antes de fechar\n🙂','Flush draft on close/reopen');
t.button('#carga',document.querySelector('.channel-list')).click(); await t.idle();
t.assert(document.querySelector('#message-input').value==='rascunho persistido do canal de carga','Other draft restored');
t.assert(document.querySelectorAll('.message-row').length===50,'Bounded page after navigation restore');
return {restored_selection:true,restored_drafts:true};
"""


def main():
    mode = sys.argv[1]
    if mode == "close":
        close_window(int(sys.argv[2]))
        return
    tools = DevTools(int(os.environ.get("BOBOCORD_TEST_DEBUG_PORT", "9223")))
    tools.evaluate(f"(async()=>{{{HELPERS}}})()")
    if mode in ("scroll", "write-close", "read"):
        scenario = {"scroll": SCROLL, "write-close": WRITE, "read": READ}[mode]
        result = tools.evaluate("(async()=>{" + scenario + "})()")
        print(json.dumps(result, ensure_ascii=True))
        if mode == "write-close":
            close_window(int(sys.argv[2]))
            return
    elif mode == "context":
        tools.evaluate("document.querySelector('.channel-button').focus()")
        tools.call("Input.dispatchKeyEvent", type="keyDown", key="F10", code="F10", windowsVirtualKeyCode=121, modifiers=8)
        tools.evaluate("(async()=>{await boboTest.wait(()=>document.querySelector('[role=menu]'));boboTest.assert(document.activeElement.getAttribute('role')==='menuitem','Menu autofocus');})()")
        tools.call("Input.dispatchKeyEvent", type="keyDown", key="ArrowDown", code="ArrowDown", windowsVirtualKeyCode=40)
        tools.evaluate("(async()=>{await boboTest.sleep(30);boboTest.assert(document.activeElement.textContent==='Excluir…','Arrow key menu navigation');})()")
        tools.call("Input.dispatchKeyEvent", type="keyDown", key="Escape", code="Escape", windowsVirtualKeyCode=27)
        tools.evaluate("(async()=>{await boboTest.wait(()=>!document.querySelector('[role=menu]'));})()")
        print('PASS: Shift+F10, menu focus, ArrowDown, Escape')
    elif mode == "probe":
        print(json.dumps(tools.evaluate("({rows:document.querySelectorAll('.message-row').length,ready:!document.querySelector('.add-space').disabled})")))
        return
    else:
        raise SystemExit("Usage: f1_smoke.py scroll|context|write-close PID|read|probe")
    tools.screenshot(f"f1-{mode}.png")


if __name__ == "__main__":
    main()
