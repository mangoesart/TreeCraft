(function () {
  var doc = document;

  // Dropdown menus: one open at a time, close on outside click or Escape.
  function closeAll(except) {
    doc.querySelectorAll('.menu.open, .search.open').forEach(function (m) {
      if (m !== except) {
        m.classList.remove('open');
        var b = m.querySelector('[aria-expanded]');
        if (b) b.setAttribute('aria-expanded', 'false');
      }
    });
  }
  doc.querySelectorAll('.menu > button, .search > button').forEach(function (btn) {
    btn.addEventListener('click', function (e) {
      e.stopPropagation();
      var box = btn.parentElement;
      var open = !box.classList.contains('open');
      closeAll(box);
      box.classList.toggle('open', open);
      btn.setAttribute('aria-expanded', String(open));
      if (open && box.classList.contains('search')) box.querySelector('input').focus();
    });
  });
  doc.addEventListener('click', function (e) {
    if (!e.target.closest('.menu-pop, .search-pop')) closeAll(null);
  });
  doc.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') { closeAll(null); setNav(false); }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      doc.querySelector('.search-btn').click();
    }
  });

  // Text size
  var sizeBtns = doc.querySelectorAll('[data-size]');
  function markSize() {
    var cur = doc.documentElement.dataset.size || 'default';
    sizeBtns.forEach(function (b) { b.setAttribute('aria-checked', String(b.dataset.size === cur)); });
  }
  sizeBtns.forEach(function (b) {
    b.addEventListener('click', function () {
      doc.documentElement.dataset.size = b.dataset.size;
      try { localStorage.setItem('tc-size', b.dataset.size); } catch (e) {}
      markSize();
      closeAll(null);
    });
  });
  markSize();

  // Phone menu
  var toggle = doc.querySelector('.nav-toggle');
  var scrim = doc.querySelector('.scrim');
  function setNav(open) {
    if (!doc.querySelector('.sidebar')) return;
    doc.body.classList.toggle('nav-open', open);
    if (toggle) toggle.setAttribute('aria-expanded', String(open));
    if (scrim) scrim.hidden = !open;
  }
  if (toggle) {
    if (!doc.querySelector('.sidebar')) toggle.style.visibility = 'hidden';
    toggle.addEventListener('click', function (e) {
      e.stopPropagation();
      setNav(!doc.body.classList.contains('nav-open'));
    });
  }
  if (scrim) scrim.addEventListener('click', function () { setNav(false); });

  // Sidebar filter
  var filter = doc.querySelector('.filter input');
  if (filter) {
    filter.addEventListener('input', function () {
      var q = filter.value.trim().toLowerCase();
      var any = false;
      doc.querySelectorAll('.nav-group').forEach(function (g) {
        var shown = 0;
        g.querySelectorAll('li').forEach(function (li) {
          var ok = !q || li.textContent.toLowerCase().indexOf(q) >= 0;
          li.hidden = !ok;
          if (ok) shown++;
        });
        g.hidden = shown === 0;
        if (shown) any = true;
      });
      doc.querySelector('.filter-empty').hidden = any;
    });
  }

  // Search
  var input = doc.querySelector('.search-pop input');
  var list = doc.querySelector('.search-results');
  var index = window.TC_INDEX || [];
  function runSearch() {
    var q = input.value.trim().toLowerCase();
    if (!q) { list.innerHTML = ''; return; }
    var words = q.split(/\s+/);
    var hits = index.map(function (p) {
      var t = p.t.toLowerCase(), all = (p.t + ' ' + p.d + ' ' + p.x).toLowerCase(), score = 0;
      for (var i = 0; i < words.length; i++) {
        if (all.indexOf(words[i]) < 0) return null;
        score += t.indexOf(words[i]) >= 0 ? 10 : 1;
      }
      return { p: p, s: score };
    }).filter(Boolean).sort(function (a, b) { return b.s - a.s; }).slice(0, 8);
    list.innerHTML = hits.length ? hits.map(function (h, i) {
      return '<li><a href="' + h.p.u + '"' + (i === 0 ? ' class="active"' : '') + '>' + h.p.t + '<small>' + h.p.g + ' · ' + h.p.d + '</small></a></li>';
    }).join('') : '<li class="none">No results for “' + input.value.replace(/</g, '&lt;') + '”</li>';
  }
  if (input) {
    input.addEventListener('input', runSearch);
    input.addEventListener('keydown', function (e) {
      if (e.key === 'Enter') { var a = list.querySelector('a'); if (a) location.href = a.href; }
    });
  }

  // "On this page": highlight the section being read
  var tocLinks = Array.prototype.slice.call(doc.querySelectorAll('.toc a'));
  if (tocLinks.length && 'IntersectionObserver' in window) {
    var heads = tocLinks.map(function (a) { return doc.getElementById(a.getAttribute('href').slice(1)); }).filter(Boolean);
    var obs = new IntersectionObserver(function () {
      var current = heads[0];
      heads.forEach(function (h) { if (h.getBoundingClientRect().top < 140) current = h; });
      tocLinks.forEach(function (a) { a.classList.toggle('on', a.getAttribute('href') === '#' + current.id); });
    }, { rootMargin: '-100px 0px -60% 0px', threshold: [0, 1] });
    heads.forEach(function (h) { obs.observe(h); });
  }
})();
