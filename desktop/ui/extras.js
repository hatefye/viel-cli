(() => {
  const V = window.Viel;
  const { api, invoke, listen, App, S, Settings, Pages, Player, openInfo, toast, el, esc, $, $$, byId, I, clock } = V;
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const store = {
    get(k, d) { try { const v = JSON.parse(localStorage.getItem(k)); return v == null ? d : v; } catch (_) { return d; } },
    set(k, v) { localStorage.setItem(k, JSON.stringify(v)); },
  };

  const Manga = (() => {
    const st = { sort: 'popular', q: '', items: [], next: null, loading: false, gen: 0, shelf: store.get('manga.shelf', []), read: store.get('manga.read', {}), view: null };
    const short = name => String(name || '').replace(/^Chapter\s+/i, 'Ch. ');
    const img = (url, cls) => `<img class="${cls || ''}" loading="lazy" referrerpolicy="no-referrer" src="${esc(url || '')}" data-src="${esc(url || '')}" alt="">`;
    const relay = url => `${App.boot.host}/api/manga/img?u=${encodeURIComponent(url)}`;
    document.addEventListener('error', e => {
      const t = e.target;
      if (!t || t.tagName !== 'IMG' || !t.dataset.src || t.dataset.tried) return;
      t.dataset.tried = '1';
      t.src = relay(t.dataset.src);
    }, true);
    const persist = () => { store.set('manga.shelf', st.shelf.slice(0, 60)); store.set('manga.read', st.read); };
    const markFor = id => st.shelf.find(m => m.id === id);

    async function syncShelf() {
      if (!App.user) return;
      try {
        const r = await api.get('manga/shelf');
        const merged = {};
        st.shelf.forEach(m => { merged[m.id] = m; });
        (r.items || []).forEach(m => { if (!merged[m.id] || (merged[m.id].updatedAt || 0) < m.updatedAt) merged[m.id] = m; });
        st.shelf = Object.values(merged).sort((a, b) => (b.updatedAt || 0) - (a.updatedAt || 0));
        persist();
      } catch (_) {}
    }

    let saveTimer = null, pendingMark = null;
    function record(ch, page) {
      const total = ch.pages.length;
      if (!total) return;
      const mark = { id: ch.title.id, title: ch.title.title, cover: ch.title.cover, chapter: ch.id, chapterName: ch.name, page, pages: total, updatedAt: Date.now() };
      st.shelf = [mark].concat(st.shelf.filter(m => m.id !== mark.id));
      if (page >= total - 1) { const set = new Set(st.read[mark.id] || []); set.add(ch.id); st.read[mark.id] = Array.from(set); }
      persist();
      if (!App.user) return;
      pendingMark = mark;
      clearTimeout(saveTimer);
      saveTimer = setTimeout(flush, page >= total - 1 ? 0 : 2500);
    }
    function flush() {
      clearTimeout(saveTimer);
      const mark = pendingMark; pendingMark = null;
      if (mark) api.post('manga/progress', mark).catch(() => {});
    }

    function cardHtml(m) {
      return `<a class="mg-card" data-id="${esc(m.id)}"><div class="mg-cover">${img(m.cover)}</div><div class="mg-name">${esc(m.title)}</div><div class="mg-sub">${esc([m.type, m.status, m.year].filter(Boolean).join(' · '))}</div></a>`;
    }
    function shelfHtml() {
      if (!st.shelf.length) return '';
      return `<section class="mg-shelf"><h2>Continue reading</h2><div class="mg-row">${st.shelf.slice(0, 20).map(m => {
        const frac = m.pages ? Math.min(1, (m.page + 1) / m.pages) : 0;
        return `<a class="mg-card resume" data-chapter="${esc(m.chapter)}" data-id="${esc(m.id)}"><button class="mg-x" title="Remove" aria-label="Remove">${I.x}</button><div class="mg-cover">${img(m.cover)}<div class="mg-bar"><i style="width:${Math.round(frac * 100)}%"></i></div></div><div class="mg-name">${esc(m.title)}</div><div class="mg-sub">${esc(short(m.chapterName))}${m.pages ? ` · ${m.page + 1}/${m.pages}` : ''}</div></a>`;
      }).join('')}</div></section>`;
    }

    async function load(reset) {
      if (st.loading && !reset) return;
      const gen = ++st.gen;
      st.loading = true;
      const offset = reset ? 0 : st.next;
      if (reset) { st.items = []; st.next = null; paintGrid(); }
      try {
        const r = await api.get('manga/list', { q: st.q, sort: st.sort, offset: String(offset || 0) });
        if (gen !== st.gen) return;
        const known = new Set(st.items.map(x => x.id));
        st.items = st.items.concat((r.items || []).filter(x => !known.has(x.id)));
        st.next = r.next == null ? null : r.next;
        st.failed = false;
      } catch (e) { if (gen !== st.gen) return; st.failed = true; }
      st.loading = false;
      paintGrid();
    }
    function paintGrid() {
      const grid = byId('mg-grid');
      if (!grid) return;
      if (!st.items.length) grid.innerHTML = `<div class="mg-note">${st.loading ? 'Loading…' : st.failed ? 'Manga could not be loaded. Check the connection and try again.' : st.q ? `Nothing matches “${esc(st.q)}”.` : 'Nothing here yet.'}</div>`;
      else grid.innerHTML = st.items.map(cardHtml).join('') + (st.next != null ? '<div class="mg-more" id="mg-more"></div>' : '');
      const more = byId('mg-more');
      if (more) new IntersectionObserver((es, o) => { if (es.some(x => x.isIntersecting)) { o.disconnect(); load(false); } }, { rootMargin: '600px' }).observe(more);
    }

    function page(content) {
      st.view = null;
      content.innerHTML = `<div class="page-head mg-head"><div><h1>Manga</h1><div class="sub">Your place follows your profile to the site and the iPhone app.</div></div><div class="mg-tools"><input id="mg-q" placeholder="Search manga" spellcheck="false" value="${esc(st.q)}"><div class="vs-seg" id="mg-sort">${[['popular', 'Popular'], ['latest', 'Latest'], ['new', 'New']].map(([v, l]) => `<button data-v="${v}" class="${st.sort === v ? 'active' : ''}">${l}</button>`).join('')}</div></div></div><div id="mg-shelf">${st.q ? '' : shelfHtml()}</div><div class="mg-grid" id="mg-grid"></div>`;
      let timer;
      byId('mg-q').oninput = e => { clearTimeout(timer); timer = setTimeout(() => { st.q = e.target.value.trim(); byId('mg-shelf').innerHTML = st.q ? '' : shelfHtml(); byId('mg-sort').style.visibility = st.q ? 'hidden' : ''; load(true); }, 350); };
      $$('#mg-sort button').forEach(b => b.onclick = () => { st.sort = b.dataset.v; $$('#mg-sort button').forEach(x => x.classList.toggle('active', x === b)); load(true); });
      content.onclick = e => {
        if (App.page !== 'manga') return;
        const x = e.target.closest('.mg-x');
        const card = e.target.closest('.mg-card');
        if (x && card) { e.preventDefault(); st.shelf = st.shelf.filter(m => m.id !== card.dataset.id); persist(); if (App.user) api.post('manga/remove', { key: card.dataset.id }).catch(() => {}); card.remove(); return; }
        if (!card) return;
        if (card.dataset.chapter) read(card.dataset.chapter);
        else openTitle(card.dataset.id, content);
      };
      if (st.items.length) paintGrid(); else load(true);
      syncShelf().then(() => { const box = byId('mg-shelf'); if (box && !st.q && App.page === 'manga' && !st.view) box.innerHTML = shelfHtml(); });
    }

    async function openTitle(id, content) {
      st.view = id;
      const known = st.items.find(x => x.id === id) || markFor(id) || { id, title: '' };
      window.scrollTo({ top: 0 });
      content.innerHTML = `<div class="mg-title"><button class="mg-back" id="mg-back">${I.left}<span>Manga</span></button><div class="mg-top"><div class="mg-cover big">${img(known.cover)}</div><div class="mg-info"><h1>${esc(known.title)}</h1><div id="mg-meta" class="mg-meta"></div><div id="mg-actions"></div><p id="mg-desc" class="mg-desc"></p></div></div><div id="mg-chapters" class="mg-chapters"><div class="mg-note">Loading chapters…</div></div></div>`;
      byId('mg-back').onclick = () => page(content);
      let t;
      try { t = await api.get('manga/title', { id }); } catch (e) { if (st.view === id) byId('mg-chapters').innerHTML = `<div class="mg-note">This title could not be loaded. ${esc(e)}</div>`; return; }
      if (st.view !== id || App.page !== 'manga') return;
      if (App.user) api.get('manga/state', { id }).then(r => {
        if (r.read && r.read.length) { st.read[id] = Array.from(new Set((st.read[id] || []).concat(r.read))); }
        if (r.progress && (!markFor(id) || (markFor(id).updatedAt || 0) < r.progress.updatedAt)) st.shelf = [r.progress].concat(st.shelf.filter(m => m.id !== id));
        persist();
        if (st.view === id) paintTitle(t, content);
      }).catch(() => {});
      paintTitle(t, content);
    }
    let oldestFirst = false;
    function paintTitle(t, content) {
      $('.mg-title h1', content).textContent = t.title;
      const cover = $('.mg-cover.big img', content);
      if (cover && t.cover && cover.dataset.src !== t.cover) { cover.dataset.src = t.cover; delete cover.dataset.tried; cover.src = t.cover; }
      byId('mg-meta').innerHTML = [(t.authors || []).slice(0, 3).join(', '), [t.type, t.status, t.year].filter(Boolean).join(' · '), (t.genres || []).slice(0, 5).join(', ')].filter(Boolean).map(x => `<div>${esc(x)}</div>`).join('');
      byId('mg-desc').textContent = t.description || '';
      const mark = markFor(t.id);
      const current = mark && t.chapters.find(c => c.id === mark.chapter);
      const target = current || t.chapters[t.chapters.length - 1];
      byId('mg-actions').innerHTML = target ? `<button class="mg-read" data-chapter="${esc(target.id)}">${current ? `Continue ${esc(short(target.name))}` : 'Start reading'}</button>` : '';
      const done = new Set(st.read[t.id] || []);
      const list = oldestFirst ? t.chapters.slice().reverse() : t.chapters;
      byId('mg-chapters').innerHTML = `<div class="mg-ch-head"><h2>${t.chapters.length} chapter${t.chapters.length === 1 ? '' : 's'}</h2><button id="mg-order">${oldestFirst ? 'Oldest first' : 'Newest first'}</button></div>` + (list.length ? list.map(c => `<a class="mg-ch${done.has(c.id) ? ' done' : ''}${mark && mark.chapter === c.id ? ' on' : ''}" data-chapter="${esc(c.id)}"><span class="n">${esc(c.name)}</span>${done.has(c.id) ? `<span class="tick">${I.check}</span>` : mark && mark.chapter === c.id && mark.pages ? `<span class="at">${mark.page + 1}/${mark.pages}</span>` : ''}<span class="d">${esc(String(c.date || '').slice(0, 10))}</span></a>`).join('') : '<div class="mg-note">No chapters yet.</div>');
      byId('mg-order').onclick = () => { oldestFirst = !oldestFirst; paintTitle(t, content); };
      content.onclick = e => {
        if (App.page !== 'manga') return;
        const c = e.target.closest('[data-chapter]');
        if (c) read(c.dataset.chapter, () => { if (st.view === t.id && App.page === 'manga') paintTitle(t, content); });
      };
    }

    const reader = { el: null, ch: null, page: 0, onClose: null, gen: 0 };
    function read(chapterId, onClose) {
      if (!reader.el) {
        reader.el = el(`<div class="mg-reader" hidden><div class="mg-rbar"><button data-r="close" title="Close" aria-label="Close">${I.x}</button><div class="mg-rt"><b></b><span></span></div><div class="mg-rnav"><button data-r="prev" title="Previous chapter">${I.left}</button><span class="mg-rp"></span><button data-r="next" title="Next chapter">${I.right}</button></div></div><div class="mg-pages"></div></div>`);
        document.body.appendChild(reader.el);
        reader.el.addEventListener('click', e => {
          const b = e.target.closest('[data-r]');
          if (!b) return;
          const r = b.dataset.r;
          if (r === 'close') closeReader();
          else if (r === 'prev' && reader.ch && reader.ch.prev) openChapter(reader.ch.prev);
          else if ((r === 'next' || r === 'nextend') && reader.ch && reader.ch.next) openChapter(reader.ch.next);
          else if (r === 'retry') openChapter(reader.wanted);
        });
        window.addEventListener('keydown', e => {
          if (reader.el.hidden) return;
          const box = $('.mg-pages', reader.el);
          if (e.key === 'Escape') { e.stopPropagation(); e.preventDefault(); closeReader(); }
          else if (e.key === 'ArrowRight' && reader.ch && reader.ch.next) openChapter(reader.ch.next);
          else if (e.key === 'ArrowLeft' && reader.ch && reader.ch.prev) openChapter(reader.ch.prev);
          else if (e.key === ' ' || e.key === 'PageDown') { e.preventDefault(); box.scrollBy({ top: box.clientHeight * 0.9, behavior: 'smooth' }); }
          else if (e.key === 'PageUp') { e.preventDefault(); box.scrollBy({ top: -box.clientHeight * 0.9, behavior: 'smooth' }); }
        }, true);
      }
      reader.onClose = onClose || null;
      reader.el.hidden = false;
      document.documentElement.classList.add('reading');
      openChapter(chapterId);
    }
    function closeReader() {
      flush();
      reader.el.hidden = true;
      reader.gen++;
      $('.mg-pages', reader.el).innerHTML = '';
      document.documentElement.classList.remove('reading');
      if (reader.onClose) reader.onClose();
      else if (App.page === 'manga' && !st.view) { const box = byId('mg-shelf'); if (box && !st.q) box.innerHTML = shelfHtml(); }
    }
    async function openChapter(id) {
      const gen = ++reader.gen;
      reader.wanted = id;
      const box = $('.mg-pages', reader.el);
      box.innerHTML = '<div class="mg-note light">Loading chapter…</div>';
      box.scrollTop = 0;
      let ch;
      try { ch = await api.get('manga/chapter', { id }); } catch (e) { if (gen === reader.gen) box.innerHTML = `<div class="mg-note light">This chapter could not be loaded.<br><button data-r="retry">Try again</button></div>`; return; }
      if (gen !== reader.gen) return;
      reader.ch = ch;
      $('.mg-rt b', reader.el).textContent = ch.title.title;
      $('.mg-rt span', reader.el).textContent = ch.name;
      $('[data-r=prev]', reader.el).disabled = !ch.prev;
      $('[data-r=next]', reader.el).disabled = !ch.next;
      const mark = markFor(ch.title.id);
      const begin = mark && mark.chapter === ch.id && mark.page < ch.pages.length - 1 ? mark.page : 0;
      box.innerHTML = ch.pages.map((u, i) => `<div class="mg-page" data-i="${i}">${img(u)}</div>`).join('') + `<div class="mg-end"><b>End of ${esc(short(ch.name))}</b>${ch.next ? '<button data-r="nextend">Next chapter</button>' : '<span>You are caught up.</span>'}</div>`;
      const count = $('.mg-rp', reader.el);
      const seen = new IntersectionObserver(entries => {
        if (gen !== reader.gen) return;
        let top = -1;
        entries.forEach(en => { if (en.isIntersecting) top = Math.max(top, Number(en.target.dataset.i)); });
        if (top < 0) return;
        reader.page = top;
        count.textContent = `${top + 1} / ${ch.pages.length}`;
        record(ch, top);
      }, { root: box, threshold: 0.35 });
      $$('.mg-page', box).forEach(p => seen.observe(p));
      count.textContent = `${begin + 1} / ${ch.pages.length}`;
      record(ch, begin);
      if (begin > 0) { await sleep(120); const target = $(`.mg-page[data-i="${begin}"]`, box); if (target && gen === reader.gen) target.scrollIntoView(); }
    }
    return { page, reset() { st.shelf = []; st.read = {}; persist(); } };
  })();
  Pages.manga = Manga.page;

  Pages.find = content => {
    content.innerHTML = `<div class="page-head"><div><h1>Find a scene</h1><div class="sub">Drop a screenshot from an anime, or search for a line someone says.</div></div></div>
      <div class="fd-cols"><section class="fd-box"><h2>From a screenshot</h2><label class="fd-drop" id="fd-drop"><input type="file" accept="image/*" id="fd-file" hidden><b>Drop an image here</b><span>or click to choose one. You can also paste with Ctrl+V.</span></label><div id="fd-shot" class="fd-out"></div></section>
      <section class="fd-box"><h2>From a line of dialogue</h2><input id="fd-q" class="fd-input" placeholder="Type a few words that were said" spellcheck="false"><div id="fd-lines" class="fd-out"><div class="fd-note">Covers shows whose subtitles have been indexed, so not every title is here.</div></div></section></div>`;
    const shot = byId('fd-shot'), lines = byId('fd-lines'), drop = byId('fd-drop');
    const go = (kind, id, name, season, episode, at) => openInfo({ kind, id: Number(id), name: name || '' }, { autoplay: true, row: { kind, id: Number(id), show: name || '', season: season || 0, episode: episode || 1, position: Math.max(at - 3, 1), duration: Math.max(at + 600, 1200) } });
    async function lookup(file) {
      if (!file || !/^image\//.test(file.type)) { shot.innerHTML = '<div class="fd-note">That is not an image.</div>'; return; }
      if (file.size > 9 * 1024 * 1024) { shot.innerHTML = '<div class="fd-note">That image is too large. Keep it under 9 MB.</div>'; return; }
      shot.innerHTML = '<div class="fd-note">Looking it up…</div>';
      const data = await new Promise(r => { const fr = new FileReader(); fr.onload = () => r(String(fr.result).split(',')[1] || ''); fr.readAsDataURL(file); });
      let rows;
      try { rows = (await api.post('trace', { image: data, mime: file.type })).results || []; } catch (e) { shot.innerHTML = `<div class="fd-note">${esc(e)}</div>`; return; }
      if (App.page !== 'find') return;
      if (!rows.length) { shot.innerHTML = '<div class="fd-note">No match. Cropped, mirrored or heavily edited frames are hard to place.</div>'; return; }
      shot.innerHTML = rows.slice(0, 6).map((r, i) => `<a class="fd-hit" data-i="${i}">${r.image ? `<img src="${esc(r.image)}" alt="">` : '<span class="ph"></span>'}<div><b>${esc(r.title || 'Unknown title')}</b><span>${r.episode != null ? `Episode ${r.episode} · ` : ''}${clock(r.at)}</span><small>${Math.round(r.similarity * 100)}% match${r.similarity < 0.87 ? ' · may be wrong' : ''}</small></div></a>`).join('');
      $$('.fd-hit', shot).forEach(a => a.onclick = () => { const r = rows[Number(a.dataset.i)]; if (r.anilistId) go('anime', r.anilistId, r.title, 0, r.episode || 1, r.at); });
    }
    byId('fd-file').onchange = e => lookup(e.target.files[0]);
    drop.ondragover = e => { e.preventDefault(); drop.classList.add('over'); };
    drop.ondragleave = () => drop.classList.remove('over');
    drop.ondrop = e => { e.preventDefault(); drop.classList.remove('over'); lookup(e.dataTransfer.files[0]); };
    content.onpaste = null;
    window.onpaste = e => { if (App.page !== 'find') return; const f = Array.from(e.clipboardData.files || []).find(x => /^image\//.test(x.type)); if (f) { e.preventDefault(); lookup(f); } };
    let timer;
    byId('fd-q').oninput = e => {
      clearTimeout(timer);
      const q = e.target.value.trim();
      timer = setTimeout(async () => {
        if (q.length < 2) { lines.innerHTML = '<div class="fd-note">Covers shows whose subtitles have been indexed, so not every title is here.</div>'; return; }
        let rows = [];
        try { rows = (await api.get('dialogue', { q })).results || []; } catch (_) {}
        if (App.page !== 'find' || byId('fd-q').value.trim() !== q) return;
        if (!rows.length) { lines.innerHTML = '<div class="fd-note">Nobody says that in the shows indexed so far.</div>'; return; }
        lines.innerHTML = rows.slice(0, 40).map((r, i) => `<a class="fd-line" data-i="${i}"><q>${esc(r.text)}</q><span>${esc(r.title || '')} · ${r.type === 'movie' ? '' : r.type === 'anime' ? `E${r.episode} · ` : `S${r.season} E${r.episode} · `}${clock(r.at)}</span></a>`).join('');
        $$('.fd-line', lines).forEach(a => a.onclick = () => { const r = rows[Number(a.dataset.i)]; go(r.type, r.id, r.title, r.season, r.episode, r.at); });
      }, 300);
    };
  };

  V.accountExtra = async (body, reopen) => {
    if (!App.user) return;
    const box = el('<div class="pf-box"><div class="vs-label">Profile</div><div class="pf-list"><span class="pf-note">Loading…</span></div><div class="pf-msg"></div></div>');
    const facts = $('.vs-facts', body);
    body.insertBefore(box, facts);
    const list = $('.pf-list', box), msg = $('.pf-msg', box);
    const act = async (payload, done) => {
      msg.textContent = '';
      try { await api.post('profiles', payload); } catch (e) { msg.textContent = String(e); return; }
      if (done) done();
      paint();
    };
    const switched = () => { App.home = null; App.watchlist = null; App.history = null; App.homeAt = 0; Manga.reset(); ['last', 'watched'].forEach(k => localStorage.removeItem(k)); V.showMode('home'); };
    async function paint() {
      let r;
      try { r = await api.get('profiles'); } catch (e) { list.innerHTML = `<span class="pf-note">${esc(e)}</span>`; return; }
      const all = r.profiles || [];
      list.innerHTML = all.map(p => `<button class="pf${p.active ? ' on' : ''}" data-id="${p.id}"><i style="background:${esc(p.color || '#8b8f99')}">${esc((p.name || '?')[0].toUpperCase())}</i><span>${esc(p.name)}</span></button>`).join('') + (all.length < (r.max || 5) ? '<button class="pf add" data-add="1"><i>+</i><span>Add</span></button>' : '');
      $$('.pf', list).forEach(b => {
        b.onclick = () => {
          if (b.dataset.add) {
            const row = el('<form class="pf-new"><input maxlength="20" placeholder="Profile name" spellcheck="false"><button>Add</button></form>');
            list.appendChild(row); b.remove(); $('input', row).focus();
            row.onsubmit = e => { e.preventDefault(); const name = $('input', row).value.trim(); if (name) act({ action: 'create', name }); };
            return;
          }
          const id = Number(b.dataset.id);
          if (!b.classList.contains('on')) act({ action: 'use', id }, () => { toast(`Switched to ${$('span', b).textContent}`); switched(); });
        };
        if (!b.dataset.add) b.oncontextmenu = e => {
          e.preventDefault();
          const id = Number(b.dataset.id), name = $('span', b).textContent;
          const items = [{ label: 'Rename', run: () => { const row = el(`<form class="pf-new"><input maxlength="20" value="${esc(name)}" spellcheck="false"><button>Save</button></form>`); list.appendChild(row); $('input', row).select(); row.onsubmit = ev => { ev.preventDefault(); const v = $('input', row).value.trim(); if (v) act({ action: 'rename', id, name: v }); }; } }];
          if (all.length > 1 && !b.classList.contains('on')) items.push({ label: 'Delete this profile', run: () => { if (b.dataset.sure) act({ action: 'remove', id }); else { b.dataset.sure = '1'; msg.textContent = `Right-click ${name} and choose Delete again to remove it and its history.`; } } });
          V.menuAt(b, items);
        };
      });
      if (!$('.pf-hint', box)) box.appendChild(el('<div class="pf-hint">Each profile keeps its own list, history and manga. Right-click one to rename or delete it.</div>'));
    }
    paint();
  };

  const Remote = (() => {
    let busy = false, last = 0;
    function open(path) {
      const m = String(path || '').match(/^\/(movies|tv|anime)\/(\d+)(?:\/(\d+))?(?:\/(\d+))?(?:\?at=(\d+))?$/);
      if (!m) return;
      const kind = m[1] === 'movies' ? 'movie' : m[1], id = Number(m[2]), at = Number(m[5]) || 0;
      const season = kind === 'tv' ? Number(m[3]) || 0 : 0, episode = kind === 'tv' ? Number(m[4]) || 0 : kind === 'anime' ? Number(m[3]) || 0 : 0;
      invoke('win', { action: 'show' }).catch(() => {});
      const row = kind === 'movie' ? (at ? { kind, id, season: 0, episode: 0, position: at, duration: at + 3600 } : null) : (episode ? { kind, id, season, episode, position: at, duration: at ? at + 1200 : 0 } : null);
      openInfo({ kind, id, name: '' }, row ? { autoplay: true, row } : { autoplay: true });
    }
    async function beat() {
      if (busy || !App.user || S.remote === false) return;
      const state = Player.remoteState();
      const gap = state.video && !state.paused ? 1000 : 1800;
      if (Date.now() - last < gap) return;
      busy = true; last = Date.now();
      try {
        const r = await api.post('remote/device', Object.assign({ id: store.get('remote.id', ''), name: 'viel on Windows' }, state));
        if (r && r.id) store.set('remote.id', r.id);
        (r && r.cmds || []).forEach(c => { if (c.cmd === 'open') open(c.path); else Player.remoteCommand(c); });
      } catch (_) { last = Date.now() + 4000; }
      busy = false;
    }
    setInterval(beat, 500);
    return { open };
  })();

  const Updates = (() => {
    const newer = (a, b) => { const x = String(a || '').split('.').map(Number), y = String(b || '').split('.').map(Number); for (let i = 0; i < 3; i++) { if ((x[i] || 0) !== (y[i] || 0)) return (x[i] || 0) > (y[i] || 0); } return false; };
    let offered = '';
    const install = version => { toast(`Installing viel ${version}. Windows will ask for permission, then viel reopens.`, 15000); invoke('update_install').catch(e => toast(String(e), 6000)); };
    listen('update', p => {
      if (p.state === 'ready') { offered = p.version; toast(`viel ${p.version} is ready. Click to install it now, or it installs next time you open viel.`, 14000, () => install(p.version)); }
      else if (p.state === 'failed') V.log('update', p.error);
    });
    async function check(manual) {
      if (!App.boot.version) return;
      let j;
      try { const r = await fetch(`${App.boot.host}/desktop/version.json`, { cache: 'no-store' }); if (!r.ok) throw new Error(r.status); j = await r.json(); }
      catch (e) { if (manual) toast('Could not check for updates.'); return; }
      if (!j || !newer(j.version, App.boot.version)) { if (manual) toast(`viel ${App.boot.version} is the latest version.`); return; }
      const url = j.url || `${App.boot.host}/desktop/viel-setup.exe`;
      if (S.autoUpdate !== false && j.sha256) { if (offered !== j.version) invoke('update_fetch', { url, version: j.version, sha256: j.sha256 }); if (manual) toast(`Downloading viel ${j.version}…`); }
      else toast(`viel ${j.version} is out. Click to download.`, 9000, () => invoke('open_external', { url }));
    }
    async function boot() {
      if (S.updates === false) return;
      if (S.autoUpdate !== false) {
        const waiting = await invoke('update_pending').catch(() => null);
        if (waiting && !Player.open) { install(waiting); return; }
      }
      setTimeout(check, 4000);
      setInterval(check, 6 * 3600 * 1000);
    }
    return { boot, check };
  })();
  V.Updates = Updates;
  Updates.boot();
})();
