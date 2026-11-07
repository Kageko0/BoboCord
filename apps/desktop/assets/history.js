// Browser-only layout bookkeeping. No domain state, SQL or render polling.
window.boboHistory?.dispose();
(() => {
    const list = document.getElementById('message-history');
    if (!list) return;
    let frame = 0;
    let channel = list.dataset.channel;
    let lastIntent = list.dataset.intent;
    let view;
    const rows = () => [...list.querySelectorAll('[data-message-id]')];
    function snapshot() {
        const top = list.getBoundingClientRect().top;
        const anchor = rows().find(row => row.getBoundingClientRect().bottom > top + 2);
        view = {
            nearEnd: list.scrollHeight - list.clientHeight - list.scrollTop <= 72,
            top: list.scrollTop,
            height: list.scrollHeight,
            id: anchor?.dataset.messageId,
            offset: anchor ? anchor.getBoundingClientRect().top - top : 0
        };
    }
    function anchorView(previous, prepend) {
        const anchor = rows().find(row => row.dataset.messageId === previous.id);
        if (anchor) {
            list.scrollTop += anchor.getBoundingClientRect().top - list.getBoundingClientRect().top - previous.offset;
        } else if (prepend) {
            list.scrollTop = previous.top + list.scrollHeight - previous.height;
        }
    }
    function settle() {
        frame = 0;
        if (!list.isConnected) return;
        const intent = list.dataset.intent;
        const reset = channel !== list.dataset.channel || (intent === 'reset' && lastIntent !== 'reset');
        if (!view || reset) list.scrollTop = list.scrollHeight;
        else if (intent === 'prepend') anchorView(view, true);
        else if (intent === 'append' && view.nearEnd) list.scrollTop = list.scrollHeight;
        else anchorView(view, false);
        channel = list.dataset.channel;
        lastIntent = intent;
        snapshot();
    }
    const observer = new MutationObserver(() => {
        if (!frame) frame = requestAnimationFrame(settle);
    });
    observer.observe(list, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-intent', 'data-channel'] });
    const onScroll = () => { if (!frame) snapshot(); };
    list.addEventListener('scroll', onScroll, {passive: true});
    const beforeAction = event => {
        if (event.target.closest('[data-history-action]')) snapshot();
    };
    list.addEventListener('click', beforeAction, true);
    list.scrollTop = list.scrollHeight;
    snapshot();
    window.boboHistory = { dispose() {
        observer.disconnect();
        list.removeEventListener('scroll', onScroll);
        list.removeEventListener('click', beforeAction, true);
        if (frame) cancelAnimationFrame(frame);
        window.boboHistory = undefined;
    }};
})();
