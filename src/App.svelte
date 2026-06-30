<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let accounts = [];
  let error = "";
  let filter = "";
  let showForm = false;

  let newPlatform = "zalo";
  let newLabel = "";
  let newUrl = "";

  const PLATFORMS = [
    { key: "zalo",   label: "Zalo",       url: "https://chat.zalo.me",    color: "#0068FF" },
    { key: "twitter", label: "Twitter (X)", url: "https://x.com",          color: "#000000" },
    { key: "instagram", label: "Instagram", url: "https://www.instagram.com", color: "#E1306C" },
    { key: "facebook", label: "Facebook",   url: "https://www.facebook.com", color: "#1877F2" },
    { key: "tiktok",  label: "TikTok",     url: "https://www.tiktok.com",  color: "#00F2EA" },
    { key: "google",  label: "Google",     url: "https://accounts.google.com", color: "#4285F4" },
  ];

  const platformMap = {};
  PLATFORMS.forEach(p => { platformMap[p.key] = p; });

  // ponytail: auto-fill URL when platform selected
  $: newUrl = platformMap[newPlatform]?.url || "";

  onMount(loadAccounts);

  async function loadAccounts() {
    try {
      error = "";
      accounts = await invoke("list_accounts");
    } catch (e) {
      error = String(e);
    }
  }

  async function addAccount() {
    if (!newLabel.trim()) return;
    try {
      error = "";
      showForm = false;
      await invoke("add_account", {
        platform: newPlatform,
        label: newLabel.trim(),
        url: newUrl.trim() || platformMap[newPlatform].url,
      });
      newLabel = "";
      newUrl = "";
      await loadAccounts();
    } catch (e) {
      error = String(e);
    }
  }

  async function removeAccount(id) {
    try {
      error = "";
      await invoke("remove_account", { id });
      await loadAccounts();
    } catch (e) {
      error = String(e);
    }
  }

  async function openAccount(id) {
    try {
      error = "";
      await invoke("open_account", { accountId: id });
    } catch (e) {
      error = String(e);
    }
  }

  // ponytail: WebKitGTK can't render Zalo/Facebook web apps — give users an escape hatch.
  async function openInBrowser(url) {
    try {
      error = "";
      await invoke("plugin:opener|open_url", { url, withBrowser: true });
    } catch (e) {
      error = String(e);
    }
  }

  $: filtered = accounts.filter(a => !filter || a.platform === filter);
  $: activePlatform = platformMap[filter];

  function platformInitial(key) {
    return key === "facebook" ? "f/" : key === "instagram" ? "ig" : key === "google" ? "G" : key.charAt(0).toUpperCase();
  }
</script>

<div class="shell">
  <nav class="sidebar">
    <div class="sidebar-top">
      <button
        class="icon-btn logo-btn"
        class:active={!filter}
        on:click={() => { filter = ""; showForm = false; }}
        title="All accounts"
      >
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
          <rect x="3" y="3" width="7" height="7" rx="1.5" fill="currentColor" opacity=".6"/>
          <rect x="14" y="3" width="7" height="7" rx="1.5" fill="currentColor" opacity=".9"/>
          <rect x="3" y="14" width="7" height="7" rx="1.5" fill="currentColor" opacity=".9"/>
          <rect x="14" y="14" width="7" height="7" rx="1.5" fill="currentColor" opacity=".6"/>
        </svg>
      </button>

      <div class="divider"></div>

      {#each PLATFORMS as p}
        <button
          class="icon-btn platform-icon"
          class:active={filter === p.key}
          on:click={() => { filter = p.key; showForm = false; }}
          title={p.label}
          style="--accent: {p.color}"
        >
          <span class="icon-letter">{platformInitial(p.key)}</span>
        </button>
      {/each}
    </div>

    <div class="sidebar-bottom">
      <div class="divider"></div>
      <button
        class="icon-btn add-btn"
        class:active={showForm}
        on:click={() => { showForm = !showForm; }}
        title="Add account"
      >
        <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
          <path d="M10 4v12M4 10h12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
  </nav>

  <main class="content">
    {#if error}
      <div class="toast error-toast">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/><path d="M5.5 5.5l5 5M10.5 5.5l-5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        <span>{error}</span>
        <button class="toast-close" on:click={() => error = ""}>&times;</button>
      </div>
    {/if}

    <header class="content-header">
      <div class="header-left">
        <h1>{activePlatform ? activePlatform.label : "All Accounts"}</h1>
        <span class="count">{filtered.length} account{filtered.length !== 1 ? "s" : ""}</span>
      </div>
    </header>

    {#if showForm}
      <section class="add-section">
        <div class="add-card">
          <h2>Add Account</h2>
          <div class="form-row">
            <select bind:value={newPlatform}>
              {#each PLATFORMS as p}
                <option value={p.key}>{p.label}</option>
              {/each}
            </select>
          </div>
          <div class="form-row">
            <input
              bind:value={newLabel}
              placeholder="Label (e.g. Personal, Work)"
            />
          </div>
          <div class="form-row">
            <input
              bind:value={newUrl}
              placeholder="URL (defaults to platform login page)"
            />
          </div>
          <div class="form-actions">
            <button class="btn btn-primary" on:click={addAccount} disabled={!newLabel.trim()}>
              Add Account
            </button>
            <button class="btn btn-ghost" on:click={() => showForm = false}>
              Cancel
            </button>
          </div>
        </div>
      </section>
    {/if}

    {#if filtered.length === 0 && !showForm}
      <div class="empty-state">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none" opacity=".3">
          <rect x="6" y="6" width="36" height="36" rx="8" stroke="currentColor" stroke-width="2"/>
          <path d="M24 16v16M16 24h16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <p>{filter ? "No accounts for this platform" : "No accounts yet"}</p>
        <button class="btn btn-primary" on:click={() => showForm = true}>
          Add your first account
        </button>
      </div>
    {:else}
      <div class="list">
        {#each filtered as account (account.id)}
          <div class="card" style="--accent: {(platformMap[account.platform] || {}).color || '#888'}">
            <div class="card-indicator"></div>
            <div class="card-body">
              <div class="card-top">
                <strong class="card-label">{account.label}</strong>
                <span class="card-platform">{(platformMap[account.platform] || {}).label || account.platform}</span>
              </div>
              <div class="card-url">{account.url}</div>
            </div>
            <div class="card-actions">
              <button class="btn btn-primary btn-sm" on:click={() => openAccount(account.id)}>
                Open
              </button>
              <button class="btn btn-ghost btn-sm" on:click={() => openInBrowser(account.url)} title="Open in system browser">
                ↗
              </button>
              <button class="btn btn-danger btn-sm" on:click={() => removeAccount(account.id)}>
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                  <path d="M2 4h10M5 4V2.5A.5.5 0 015.5 2h3a.5.5 0 01.5.5V4M3 4v7.5A1.5 1.5 0 004.5 13h5a1.5 1.5 0 001.5-1.5V4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </main>
</div>

<style>
  .shell {
    display: flex;
    height: 100vh;
  }

  /* ── Sidebar ── */
  .sidebar {
    width: 4.5rem;
    min-width: 4.5rem;
    background: #161618;
    border-right: 1px solid #222225;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 0;
    gap: 0.25rem;
    z-index: 10;
  }

  .sidebar-top,
  .sidebar-bottom {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.35rem;
    width: 100%;
  }

  .divider {
    width: 1.75rem;
    height: 1px;
    background: #2a2a2e;
    margin: 0.375rem 0;
  }

  .icon-btn {
    width: 2.75rem;
    height: 2.75rem;
    border: none;
    border-radius: 0.75rem;
    background: transparent;
    color: #68686e;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    position: relative;
    flex-shrink: 0;
  }

  .icon-btn:hover {
    background: #222225;
    color: #c8c8cc;
  }

  .icon-btn.active {
    background: #222225;
    color: #fff;
  }

  .icon-btn.active::before {
    content: "";
    position: absolute;
    left: -0.75rem;
    top: 50%;
    transform: translateY(-50%);
    width: 0.1875rem;
    height: 1.25rem;
    border-radius: 0 2px 2px 0;
    background: var(--accent, #888);
  }

  /* ── Platform icon buttons (colored circles) ── */
  .platform-icon {
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent, #888) 18%, transparent);
  }
  .platform-icon:hover {
    background: color-mix(in srgb, var(--accent, #888) 35%, transparent);
    color: #fff;
  }
  .platform-icon.active {
    background: color-mix(in srgb, var(--accent, #888) 85%, #000);
    color: #fff;
  }

  .logo-btn {
    color: #aaa;
  }
  .logo-btn.active::before {
    background: #888;
  }

  .icon-letter {
    font-size: 1.125rem;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: var(--accent, inherit);
  }
  .platform-icon .icon-letter {
    color: #ddd;
  }
  .platform-icon.active .icon-letter,
  .platform-icon:hover .icon-letter {
    color: #fff;
  }

  .logo-btn svg {
    width: 1.5em;
    height: 1.5em;
  }

  .add-btn svg {
    width: 1.25em;
    height: 1.25em;
  }

  .add-btn {
    color: #4a4a50;
  }
  .add-btn:hover,
  .add-btn.active {
    color: #4ade80;
  }
  .add-btn.active::before {
    background: #4ade80;
  }

  /* ── Content ── */
  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  .content-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1.25rem 1.75rem 0;
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
  }

  .content-header h1 {
    font-size: 1.25rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .count {
    font-size: 0.8125rem;
    color: #68686e;
    font-weight: 500;
  }

  /* ── Toast ── */
  .toast {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 1rem 1.75rem 0;
    padding: 0.625rem 0.875rem;
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    font-weight: 500;
    flex-shrink: 0;
  }
  .toast svg {
    width: 1em;
    height: 1em;
    flex-shrink: 0;
  }
  .error-toast {
    background: #2a1015;
    color: #f87171;
    border: 1px solid #3f1a20;
  }
  .toast-close {
    margin-left: auto;
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 1.125rem;
    line-height: 1;
    padding: 0 0.25rem;
    opacity: 0.6;
  }
  .toast-close:hover {
    opacity: 1;
  }

  /* ── Add section ── */
  .add-section {
    padding: 1rem 1.75rem 0;
    flex-shrink: 0;
  }

  .add-card {
    background: #1c1c1f;
    border: 1px solid #2a2a2e;
    border-radius: 0.75rem;
    padding: 1.25rem;
  }

  .add-card h2 {
    font-size: 0.875rem;
    font-weight: 600;
    color: #a0a0a6;
    margin-bottom: 0.875rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .form-row {
    margin-bottom: 0.625rem;
  }
  .form-row:last-of-type {
    margin-bottom: 0.875rem;
  }

  /* ponytail: Native <select> popup uses OS theme colors. appearance:none forces the
     closed-state to use our colors. For the dropdown popup we also set explicit options. */
  select {
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 16 16' fill='none'%3E%3Cpath d='M4 6l4 4 4-4' stroke='%23888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.75rem center;
    background-size: 1em;
    padding-right: 2.5rem;
  }

  select, input {
    width: 100%;
    padding: 0.625rem 0.75rem;
    border: 1px solid #2a2a2e;
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    font-family: inherit;
    background: #1a1a1d;
    color: #e8e8ea;
    outline: none;
    transition: border-color 0.15s;
  }
  select:focus, input:focus {
    border-color: #4a4a50;
  }

  /* ponytail: WebKitGTK respects these for the native dropdown popup */
  select option {
    background: #1a1a1d;
    color: #e8e8ea;
    padding: 0.25rem 0.5rem;
  }
  select option:hover {
    background: #3b82f6;
    color: #fff;
  }

  .form-actions {
    display: flex;
    gap: 0.5rem;
  }

  /* ── Buttons ── */
  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    font-weight: 600;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.12s ease;
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
  }
  .btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .btn-primary {
    background: #3b82f6;
    color: #fff;
  }
  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .btn-ghost {
    background: transparent;
    color: #68686e;
  }
  .btn-ghost:hover {
    background: #1c1c1f;
    color: #a0a0a6;
  }

  .btn-danger {
    background: transparent;
    color: #68686e;
  }
  .btn-danger:hover {
    background: #2a1015;
    color: #f87171;
  }
  .btn-danger svg {
    width: 0.875em;
    height: 0.875em;
  }

  .btn-sm {
    padding: 0.375rem 0.625rem;
    font-size: 0.75rem;
    border-radius: 0.375rem;
  }

  /* ── Empty state ── */
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    color: #68686e;
    font-size: 0.875rem;
  }
  .empty-state svg {
    width: 3em;
    height: 3em;
  }

  /* ── Account list ── */
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0.75rem 1.75rem 1.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .card {
    display: flex;
    align-items: center;
    gap: 0;
    background: #161618;
    border: 1px solid #222225;
    border-radius: 0.625rem;
    overflow: hidden;
    transition: border-color 0.15s;
  }
  .card:hover {
    border-color: #2a2a2e;
  }

  .card-indicator {
    width: 0.1875rem;
    min-width: 0.1875rem;
    align-self: stretch;
    background: var(--accent, #888);
    opacity: 0.5;
  }

  .card-body {
    flex: 1;
    padding: 0.75rem 1rem;
    min-width: 0;
  }

  .card-top {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.125rem;
  }

  .card-label {
    font-size: 0.875rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-platform {
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--accent, #888);
    background: color-mix(in srgb, var(--accent, #888) 12%, transparent);
    padding: 0.0625rem 0.5rem;
    border-radius: 0.25rem;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .card-url {
    font-size: 0.75rem;
    color: #58585e;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-actions {
    display: flex;
    gap: 0.25rem;
    padding: 0 0.75rem 0 0;
    flex-shrink: 0;
  }

  /* ── Scrollbar ── */
  .list::-webkit-scrollbar {
    width: 0.375rem;
  }
  .list::-webkit-scrollbar-track {
    background: transparent;
  }
  .list::-webkit-scrollbar-thumb {
    background: #2a2a2e;
    border-radius: 0.1875rem;
  }
  .list::-webkit-scrollbar-thumb:hover {
    background: #3a3a3e;
  }
</style>
