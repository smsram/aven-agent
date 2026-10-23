import React, { useState, useEffect, useMemo } from 'react';
import {
  Blocks,
  Compass,
  Search,
  Terminal,
  Globe,
  Cpu,
  Shield,
  Layers,
  PowerOff,
  Settings as SettingsIcon,
  Filter,
  RotateCw,
  AlertCircle,
} from 'lucide-react';
import {
  CatalogEntry,
  PluginMetadata,
  PluginKind,
  PluginStatus,
} from '../../types';
import { pluginStore } from '../../services/pluginStore';
import { PluginCatalogModal } from './PluginCatalogModal';
import { PluginDetailModal } from './PluginDetailModal';
import { IntegrationIcon } from '../../components/icons/IntegrationIcon';

export const PluginsPage: React.FC = () => {
  const [catalog, setCatalog] = useState<CatalogEntry[]>(pluginStore.getCatalog());
  const [plugins, setPlugins] = useState<PluginMetadata[]>(pluginStore.getPlugins());
  const [activeCategory, setActiveCategory] = useState<'all' | PluginKind>('all');
  const [statusFilter, setStatusFilter] = useState<'all' | 'connected' | 'disconnected' | 'needs_setup' | 'error'>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [isCatalogOpen, setIsCatalogOpen] = useState(false);
  const [selectedEntry, setSelectedEntry] = useState<{
    catalog?: CatalogEntry | null;
    plugin?: PluginMetadata | null;
  } | null>(null);

  // Subscribe to plugin store changes
  useEffect(() => {
    const unsub = pluginStore.subscribe(() => {
      setCatalog(pluginStore.getCatalog());
      setPlugins(pluginStore.getPlugins());
    });
    pluginStore.refresh();
    return unsub;
  }, []);

  const [connectingId, setConnectingId] = useState<string | null>(null);

  // Configured integrations in Aven (persisted MCP servers, configured API integrations, enabled native capabilities)
  const activeIntegrations = useMemo(() => {
    return plugins;
  }, [plugins]);

  // Apply Category, Status, and Search filters
  const filteredIntegrations = useMemo(() => {
    return activeIntegrations.filter((plugin) => {
      // Category filter
      if (activeCategory !== 'all' && plugin.kind !== activeCategory) {
        return false;
      }

      // Status filter
      if (statusFilter !== 'all') {
        if (statusFilter === 'connected' && plugin.status !== 'connected') return false;
        if (statusFilter === 'disconnected' && plugin.status !== 'disconnected') return false;
        if (statusFilter === 'needs_setup' && plugin.status !== 'needs_permission') return false;
        if (statusFilter === 'error' && plugin.status !== 'error') return false;
      }

      // Search filter
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase().trim();
        const catalogEntry = catalog.find((c) => c.id === plugin.id);
        const nameMatch = plugin.name.toLowerCase().includes(query);
        const descMatch = (catalogEntry?.description || '').toLowerCase().includes(query);
        const catMatch = (catalogEntry?.category || '').toLowerCase().includes(query);
        const capMatch = (catalogEntry?.supported_capabilities || []).some((c) => c.toLowerCase().includes(query));
        const toolMatch = (plugin.discovered_tools || []).some((t) => t.name.toLowerCase().includes(query));

        return nameMatch || descMatch || catMatch || capMatch || toolMatch;
      }

      return true;
    });
  }, [activeIntegrations, activeCategory, statusFilter, searchQuery, catalog]);

  const handleOpenDetail = (plugin: PluginMetadata) => {
    const catEntry = catalog.find((c) => c.id === plugin.id) || null;
    setSelectedEntry({
      catalog: catEntry,
      plugin,
    });
  };

  const handleSelectFromCatalog = (entry: CatalogEntry) => {
    const existingPlugin = plugins.find((p) => p.id === entry.id) || null;
    setIsCatalogOpen(false);
    setSelectedEntry({
      catalog: entry,
      plugin: existingPlugin,
    });
  };

  const handleConnect = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    setConnectingId(id);
    try {
      await pluginStore.connect(id);
    } catch (err) {
      console.error('Failed to connect plugin:', err);
    } finally {
      setConnectingId(null);
    }
  };

  const handleDisconnect = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    try {
      await pluginStore.disconnect(id);
    } catch (err) {
      console.error('Failed to disconnect plugin:', err);
    }
  };

  const getStatusInfo = (status: PluginStatus) => {
    switch (status) {
      case 'connected':
        return {
          label: 'Connected',
          color: '#34D399',
          bg: 'rgba(52, 211, 153, 0.12)',
          pulse: true,
        };
      case 'connecting':
        return {
          label: 'Connecting...',
          color: '#FBBF24',
          bg: 'rgba(251, 191, 36, 0.12)',
          pulse: true,
        };
      case 'needs_permission':
        return {
          label: 'Needs Setup',
          color: '#FBBF24',
          bg: 'rgba(251, 191, 36, 0.12)',
          pulse: false,
        };
      case 'error':
        return {
          label: 'Error',
          color: '#F87171',
          bg: 'rgba(248, 113, 113, 0.12)',
          pulse: false,
        };
      case 'unsupported':
        return {
          label: 'Unsupported OS',
          color: '#71717A',
          bg: 'rgba(113, 113, 122, 0.12)',
          pulse: false,
        };
      case 'disconnected':
      default:
        return {
          label: 'Disconnected',
          color: '#A5A6B0',
          bg: 'rgba(165, 166, 176, 0.12)',
          pulse: false,
        };
    }
  };

  const getKindInfo = (kind: PluginKind) => {
    switch (kind) {
      case 'mcp':
        return { label: 'MCP SERVER', color: '#7C7CF8', icon: <Terminal size={12} /> };
      case 'api':
        return { label: 'API INTEGRATION', color: '#38BDF8', icon: <Globe size={12} /> };
      case 'native':
        return { label: 'NATIVE CAPABILITY', color: '#34D399', icon: <Cpu size={12} /> };
    }
  };

  return (
    <div
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
        backgroundColor: '#191A1F',
      }}
    >
      {/* Top Header Bar */}
      <div
        style={{
          padding: '12px 24px',
          borderBottom: '1px solid #34353C',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          backgroundColor: '#17181D',
          flexShrink: 0,
        }}
      >
        <div>
          <h1 style={{ fontSize: '14px', fontWeight: 600, color: '#F4F4F5', margin: 0 }}>
            Plugins & Capabilities
          </h1>
          <p style={{ fontSize: '11px', color: '#A5A6B0', margin: '2px 0 0 0' }}>
            Active Model Context Protocol (MCP) servers, API integrations, and native capabilities
          </p>
        </div>

        {/* Primary CTA: Get Plugins Button */}
        <button
          type="button"
          onClick={() => setIsCatalogOpen(true)}
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '6px',
            padding: '6px 14px',
            borderRadius: '6px',
            backgroundColor: '#7C7CF8',
            border: 'none',
            color: '#FFFFFF',
            fontSize: '12px',
            fontWeight: 500,
            cursor: 'pointer',
            transition: 'background-color 0.15s',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = '#6B6BF7';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = '#7C7CF8';
          }}
        >
          <Compass size={14} />
          <span>Get Plugins</span>
        </button>
      </div>

      {/* Control Bar: Categories, Status, and Search Filters */}
      <div
        style={{
          padding: '10px 24px',
          borderBottom: '1px solid #2D2E35',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '16px',
          backgroundColor: '#1E1F24',
          flexShrink: 0,
        }}
      >
        {/* Three Category Filters + All */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          <button
            type="button"
            onClick={() => setActiveCategory('all')}
            style={{
              padding: '4px 10px',
              borderRadius: '5px',
              fontSize: '11.5px',
              fontWeight: activeCategory === 'all' ? 500 : 400,
              color: activeCategory === 'all' ? '#F4F4F5' : '#A5A6B0',
              backgroundColor: activeCategory === 'all' ? '#34353C' : 'transparent',
              border: 'none',
              cursor: 'pointer',
            }}
          >
            All ({activeIntegrations.length})
          </button>
          <button
            type="button"
            onClick={() => setActiveCategory('mcp')}
            style={{
              padding: '4px 10px',
              borderRadius: '5px',
              fontSize: '11.5px',
              fontWeight: activeCategory === 'mcp' ? 500 : 400,
              color: activeCategory === 'mcp' ? '#7C7CF8' : '#A5A6B0',
              backgroundColor: activeCategory === 'mcp' ? 'rgba(124, 124, 248, 0.15)' : 'transparent',
              border: 'none',
              cursor: 'pointer',
            }}
          >
            MCP Servers ({activeIntegrations.filter((p) => p.kind === 'mcp').length})
          </button>
          <button
            type="button"
            onClick={() => setActiveCategory('api')}
            style={{
              padding: '4px 10px',
              borderRadius: '5px',
              fontSize: '11.5px',
              fontWeight: activeCategory === 'api' ? 500 : 400,
              color: activeCategory === 'api' ? '#38BDF8' : '#A5A6B0',
              backgroundColor: activeCategory === 'api' ? 'rgba(56, 189, 248, 0.15)' : 'transparent',
              border: 'none',
              cursor: 'pointer',
            }}
          >
            API Integrations ({activeIntegrations.filter((p) => p.kind === 'api').length})
          </button>
          <button
            type="button"
            onClick={() => setActiveCategory('native')}
            style={{
              padding: '4px 10px',
              borderRadius: '5px',
              fontSize: '11.5px',
              fontWeight: activeCategory === 'native' ? 500 : 400,
              color: activeCategory === 'native' ? '#34D399' : '#A5A6B0',
              backgroundColor: activeCategory === 'native' ? 'rgba(52, 211, 153, 0.15)' : 'transparent',
              border: 'none',
              cursor: 'pointer',
            }}
          >
            Native Capabilities ({activeIntegrations.filter((p) => p.kind === 'native').length})
          </button>
        </div>

        {/* Right side: Status Filter & Search */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          {/* Status filter */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Filter size={12} color="#71717A" />
            <select
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as any)}
              style={{
                backgroundColor: '#17181D',
                border: '1px solid #34353C',
                borderRadius: '5px',
                color: '#D4D4D8',
                fontSize: '11.5px',
                padding: '4px 8px',
                outline: 'none',
                cursor: 'pointer',
              }}
            >
              <option value="all">All Status</option>
              <option value="connected">Connected</option>
              <option value="disconnected">Disconnected</option>
              <option value="needs_setup">Needs Setup</option>
              <option value="error">Error</option>
            </select>
          </div>

          {/* Search Box */}
          <div style={{ position: 'relative', width: '220px' }}>
            <Search
              size={13}
              color="#71717A"
              style={{ position: 'absolute', left: '9px', top: '50%', transform: 'translateY(-50%)' }}
            />
            <input
              type="text"
              placeholder="Filter active integrations..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={{
                width: '100%',
                padding: '5px 8px 5px 28px',
                backgroundColor: '#17181D',
                border: '1px solid #34353C',
                borderRadius: '5px',
                fontSize: '11.5px',
                color: '#F4F4F5',
                outline: 'none',
              }}
            />
          </div>
        </div>
      </div>

      {/* Main Content Area */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '24px' }}>
        {activeIntegrations.length === 0 ? (
          /* Clean Empty State as strictly required when nothing is connected or enabled */
          <div
            style={{
              maxWidth: '480px',
              margin: '80px auto',
              textAlign: 'center',
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              padding: '36px 24px',
              backgroundColor: '#25262C',
              border: '1px solid #34353C',
              borderRadius: '8px',
            }}
          >
            <div
              style={{
                width: '42px',
                height: '42px',
                borderRadius: '8px',
                backgroundColor: '#1E1F24',
                border: '1px solid #34353C',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: '#7C7CF8',
                marginBottom: '16px',
              }}
            >
              <Blocks size={22} strokeWidth={1.5} />
            </div>

            <h2 style={{ fontSize: '15px', fontWeight: 600, color: '#F4F4F5', marginBottom: '6px' }}>
              No active plugins connected
            </h2>

            <p style={{ fontSize: '12px', color: '#A5A6B0', lineHeight: 1.5, marginBottom: '22px', maxWidth: '380px' }}>
              Extend Aven with Model Context Protocol (MCP) servers, official Google Search grounding via Gemini API, or safe native Windows capabilities.
            </p>

            <button
              type="button"
              onClick={() => setIsCatalogOpen(true)}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '6px',
                padding: '8px 16px',
                borderRadius: '6px',
                backgroundColor: '#7C7CF8',
                border: 'none',
                color: '#FFFFFF',
                fontSize: '12.5px',
                fontWeight: 500,
                cursor: 'pointer',
              }}
            >
              <Compass size={14} />
              <span>Get Plugins</span>
            </button>
          </div>
        ) : filteredIntegrations.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '60px 20px', color: '#A5A6B0', fontSize: '13px' }}>
            No active integrations match your current filter or search criteria.
          </div>
        ) : (
          /* List of actually connected/enabled integrations */
          <div style={{ maxWidth: '840px', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '10px' }}>
            {filteredIntegrations.map((plugin) => {
              const catEntry = catalog.find((c) => c.id === plugin.id);
              const kindInfo = getKindInfo(plugin.kind);
              const toolCount = plugin.discovered_tools.length || catEntry?.supported_capabilities.length || 0;

              return (
                <div
                  key={plugin.id}
                  onClick={() => handleOpenDetail(plugin)}
                  style={{
                    backgroundColor: '#25262C',
                    border: '1px solid #34353C',
                    borderRadius: '8px',
                    padding: '14px 18px',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    cursor: 'pointer',
                    transition: 'border-color 0.15s',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = '#7C7CF8';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = '#34353C';
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'flex-start', gap: '14px' }}>
                    {/* Icon container */}
                    <div
                      style={{
                        width: '34px',
                        height: '34px',
                        borderRadius: '6px',
                        backgroundColor: '#1E1F24',
                        border: '1px solid #34353C',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        flexShrink: 0,
                      }}
                    >
                      <IntegrationIcon id={plugin.id} name={plugin.name} type={plugin.kind} size={18} />
                    </div>

                    <div>
                      {/* Name & Badges */}
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <span style={{ fontSize: '13.5px', fontWeight: 600, color: '#F4F4F5' }}>
                          {plugin.name}
                        </span>

                        <span
                          style={{
                            fontSize: '9.5px',
                            fontWeight: 600,
                            padding: '1px 6px',
                            borderRadius: '3px',
                            color: kindInfo.color,
                            backgroundColor: 'rgba(124, 124, 248, 0.1)',
                            border: `1px solid ${kindInfo.color}40`,
                            fontFamily: 'var(--font-mono)',
                          }}
                        >
                          {kindInfo.label}
                        </span>

                        {/* Status badge */}
                        {(() => {
                          const statusInfo = getStatusInfo(plugin.status);
                          return (
                            <span
                              style={{
                                display: 'inline-flex',
                                alignItems: 'center',
                                gap: '4px',
                                fontSize: '11px',
                                color: statusInfo.color,
                                backgroundColor: statusInfo.bg,
                                padding: '1px 6px',
                                borderRadius: '3px',
                              }}
                            >
                              <span
                                style={{
                                  width: '5px',
                                  height: '5px',
                                  borderRadius: '50%',
                                  backgroundColor: statusInfo.color,
                                  boxShadow: statusInfo.pulse ? `0 0 6px ${statusInfo.color}` : 'none',
                                }}
                                className={statusInfo.pulse ? 'animate-pulse' : ''}
                              />
                              {statusInfo.label}
                            </span>
                          );
                        })()}
                      </div>

                      {/* Description */}
                      <p style={{ fontSize: '12px', color: '#A5A6B0', margin: '4px 0 6px 0', lineHeight: 1.4 }}>
                        {catEntry?.description || 'Active integration connected to Aven.'}
                      </p>

                      {/* Error details if connection failed */}
                      {plugin.error_message && (
                        <div
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: '6px',
                            marginBottom: '6px',
                            fontSize: '11px',
                            color: '#F87171',
                          }}
                        >
                          <AlertCircle size={12} style={{ flexShrink: 0 }} />
                          <span>{plugin.error_message}</span>
                        </div>
                      )}

                      {/* Capabilities pill count */}
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '11px', color: '#71717A' }}>
                        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
                          <Layers size={11} color="#A5A6B0" />
                          <span>{toolCount} capability/tool(s) available</span>
                        </span>
                        {plugin.approved_paths.length > 0 && (
                          <>
                            <span>•</span>
                            <span style={{ display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
                              <Shield size={11} color="#34D399" />
                              <span>Scoped to approved workspace</span>
                            </span>
                          </>
                        )}
                      </div>
                    </div>
                  </div>

                  {/* Actions on the right */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    {plugin.status === 'connected' ? (
                      <button
                        type="button"
                        onClick={(e) => handleDisconnect(e, plugin.id)}
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '5px',
                          padding: '5px 8px',
                          borderRadius: '5px',
                          backgroundColor: 'transparent',
                          border: '1px solid transparent',
                          color: '#A5A6B0',
                          fontSize: '11.5px',
                          cursor: 'pointer',
                        }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.color = '#F87171';
                          e.currentTarget.style.borderColor = '#F8717130';
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.color = '#A5A6B0';
                          e.currentTarget.style.borderColor = 'transparent';
                        }}
                        title="Disconnect integration"
                      >
                        <PowerOff size={12} />
                        <span>Disconnect</span>
                      </button>
                    ) : plugin.status === 'connecting' || connectingId === plugin.id ? (
                      <button
                        type="button"
                        disabled
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '5px',
                          padding: '5px 10px',
                          borderRadius: '5px',
                          backgroundColor: '#1E1F24',
                          border: '1px solid #34353C',
                          color: '#FBBF24',
                          fontSize: '11.5px',
                          cursor: 'not-allowed',
                        }}
                      >
                        <RotateCw size={12} className="animate-spin" />
                        <span>Connecting...</span>
                      </button>
                    ) : (
                      <button
                        type="button"
                        onClick={(e) => handleConnect(e, plugin.id)}
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '5px',
                          padding: '5px 10px',
                          borderRadius: '5px',
                          backgroundColor: 'rgba(124, 124, 248, 0.15)',
                          border: '1px solid rgba(124, 124, 248, 0.35)',
                          color: '#7C7CF8',
                          fontSize: '11.5px',
                          cursor: 'pointer',
                        }}
                        title={plugin.status === 'error' ? 'Retry connection' : 'Connect integration'}
                      >
                        <RotateCw size={12} />
                        <span>{plugin.status === 'error' ? 'Retry Connection' : 'Connect'}</span>
                      </button>
                    )}

                    <button
                      type="button"
                      onClick={() => handleOpenDetail(plugin)}
                      style={{
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: '5px',
                        padding: '5px 10px',
                        borderRadius: '5px',
                        backgroundColor: '#1E1F24',
                        border: '1px solid #34353C',
                        color: '#E4E4E7',
                        fontSize: '11.5px',
                        cursor: 'pointer',
                      }}
                    >
                      <SettingsIcon size={12} />
                      <span>Configure</span>
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Integration Catalog Modal (Get Plugins) */}
      <PluginCatalogModal
        isOpen={isCatalogOpen}
        onClose={() => setIsCatalogOpen(false)}
        catalog={catalog}
        configuredPlugins={plugins}
        onSelectEntry={handleSelectFromCatalog}
      />

      {/* In-app Plugin Details Modal */}
      <PluginDetailModal
        isOpen={Boolean(selectedEntry)}
        onClose={() => setSelectedEntry(null)}
        catalogEntry={selectedEntry?.catalog}
        pluginMetadata={selectedEntry?.plugin}
        onRefresh={() => pluginStore.refresh()}
      />
    </div>
  );
};
