import React, { useState, useMemo } from 'react';
import {
  X,
  Search,
  Terminal,
  Globe,
  Cpu,
  ArrowRight,
  ShieldCheck,
  Clock,
  Compass,
} from 'lucide-react';
import { CatalogEntry, PluginMetadata, PluginKind } from '../../types';
import { IntegrationIcon } from '../../components/icons/IntegrationIcon';

export interface PluginCatalogModalProps {
  isOpen: boolean;
  onClose: () => void;
  catalog: CatalogEntry[];
  configuredPlugins: PluginMetadata[];
  onSelectEntry: (entry: CatalogEntry) => void;
}

export const PluginCatalogModal: React.FC<PluginCatalogModalProps> = ({
  isOpen,
  onClose,
  catalog,
  configuredPlugins,
  onSelectEntry,
}) => {
  const [activeCategory, setActiveCategory] = useState<'all' | PluginKind>('all');
  const [searchQuery, setSearchQuery] = useState('');

  const configuredMap = useMemo(() => {
    const map = new Map<string, PluginMetadata>();
    for (const p of configuredPlugins) {
      map.set(p.id, p);
    }
    return map;
  }, [configuredPlugins]);

  const filteredEntries = useMemo(() => {
    return catalog.filter((entry) => {
      // Category filter
      if (activeCategory !== 'all' && entry.kind !== activeCategory) {
        return false;
      }

      // Search filter
      if (searchQuery.trim()) {
        const query = searchQuery.toLowerCase().trim();
        const nameMatch = entry.name.toLowerCase().includes(query);
        const descMatch = entry.description.toLowerCase().includes(query);
        const catMatch = entry.category.toLowerCase().includes(query);
        const capMatch = entry.supported_capabilities.some((c) => c.toLowerCase().includes(query));
        return nameMatch || descMatch || catMatch || capMatch;
      }

      return true;
    });
  }, [catalog, activeCategory, searchQuery]);

  if (!isOpen) return null;

  const getKindBadge = (kind: PluginKind) => {
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
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.65)',
        backdropFilter: 'blur(3px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 999,
        padding: '24px',
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        style={{
          width: '100%',
          maxWidth: '820px',
          maxHeight: '85vh',
          backgroundColor: '#1E1F24',
          border: '1px solid #34353C',
          borderRadius: '10px',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          boxShadow: '0 20px 40px rgba(0,0,0,0.5)',
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: '16px 20px',
            borderBottom: '1px solid #34353C',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            backgroundColor: '#25262C',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Compass size={18} color="#7C7CF8" />
            <div>
              <h2 style={{ fontSize: '15px', fontWeight: 600, color: '#F4F4F5', margin: 0 }}>
                Integration Catalog
              </h2>
              <p style={{ fontSize: '11px', color: '#A5A6B0', margin: '2px 0 0 0' }}>
                Discover and configure Model Context Protocol (MCP) servers, API providers, and native capabilities
              </p>
            </div>
          </div>

          <button
            type="button"
            onClick={onClose}
            style={{
              background: 'transparent',
              border: 'none',
              color: '#A5A6B0',
              cursor: 'pointer',
              padding: '4px',
            }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Filter & Search Bar */}
        <div
          style={{
            padding: '12px 20px',
            borderBottom: '1px solid #2D2E35',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '12px',
            backgroundColor: '#1A1B20',
          }}
        >
          {/* Category Tabs */}
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
              All
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
                backgroundColor: activeCategory === 'mcp' ? 'rgba(124, 124, 248, 0.12)' : 'transparent',
                border: 'none',
                cursor: 'pointer',
              }}
            >
              MCP Servers
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
                backgroundColor: activeCategory === 'api' ? 'rgba(56, 189, 248, 0.12)' : 'transparent',
                border: 'none',
                cursor: 'pointer',
              }}
            >
              API Integrations
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
                backgroundColor: activeCategory === 'native' ? 'rgba(52, 211, 153, 0.12)' : 'transparent',
                border: 'none',
                cursor: 'pointer',
              }}
            >
              Native Capabilities
            </button>
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
              placeholder="Search catalog..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={{
                width: '100%',
                padding: '5px 8px 5px 28px',
                backgroundColor: '#15161A',
                border: '1px solid #34353C',
                borderRadius: '5px',
                fontSize: '11.5px',
                color: '#F4F4F5',
                outline: 'none',
              }}
            />
          </div>
        </div>

        {/* Catalog Grid */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '16px 20px' }}>
          {filteredEntries.length === 0 ? (
            <div style={{ textAlign: 'center', padding: '40px 20px', color: '#A5A6B0', fontSize: '13px' }}>
              No catalog entries match your search criteria.
            </div>
          ) : (
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(350px, 1fr))', gap: '12px' }}>
              {filteredEntries.map((entry) => {
                const conf = configuredMap.get(entry.id);
                const isConnected = conf?.status === 'connected';
                const isConfigured = Boolean(conf && conf.status !== 'not_configured');
                const badge = getKindBadge(entry.kind);

                return (
                  <div
                    key={entry.id}
                    onClick={() => onSelectEntry(entry)}
                    style={{
                      backgroundColor: '#25262C',
                      border: '1px solid #34353C',
                      borderRadius: '8px',
                      padding: '14px',
                      display: 'flex',
                      flexDirection: 'column',
                      justifyContent: 'space-between',
                      cursor: 'pointer',
                      transition: 'border-color 0.15s, transform 0.15s',
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.borderColor = '#7C7CF8';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.borderColor = '#34353C';
                    }}
                  >
                    <div>
                      {/* Top Row: Type & Status */}
                      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '8px' }}>
                        <span
                          style={{
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '4px',
                            fontSize: '9.5px',
                            fontWeight: 600,
                            fontFamily: 'var(--font-mono)',
                            color: badge.color,
                          }}
                        >
                          {badge.icon}
                          {badge.label}
                        </span>

                        {isConnected ? (
                          <span
                            style={{
                              display: 'inline-flex',
                              alignItems: 'center',
                              gap: '4px',
                              fontSize: '10.5px',
                              color: '#34D399',
                              backgroundColor: 'rgba(52, 211, 153, 0.12)',
                              padding: '2px 6px',
                              borderRadius: '3px',
                            }}
                          >
                            <ShieldCheck size={11} />
                            Connected
                          </span>
                        ) : isConfigured ? (
                          <span
                            style={{
                              fontSize: '10.5px',
                              color: '#A5A6B0',
                              backgroundColor: 'rgba(165, 166, 176, 0.12)',
                              padding: '2px 6px',
                              borderRadius: '3px',
                            }}
                          >
                            Configured
                          </span>
                        ) : !entry.is_implemented ? (
                          <span
                            style={{
                              display: 'inline-flex',
                              alignItems: 'center',
                              gap: '4px',
                              fontSize: '10.5px',
                              color: '#FBBF24',
                              backgroundColor: 'rgba(251, 191, 36, 0.1)',
                              padding: '2px 6px',
                              borderRadius: '3px',
                            }}
                          >
                            <Clock size={11} />
                            Planned
                          </span>
                        ) : (
                          <span
                            style={{
                              fontSize: '10.5px',
                              color: '#7C7CF8',
                              backgroundColor: 'rgba(124, 124, 248, 0.1)',
                              padding: '2px 6px',
                              borderRadius: '3px',
                            }}
                          >
                            Available
                          </span>
                        )}
                      </div>

                      {/* Name & Description */}
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '6px' }}>
                        <div
                          style={{
                            width: '26px',
                            height: '26px',
                            borderRadius: '5px',
                            backgroundColor: '#1E1F24',
                            border: '1px solid #34353C',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            flexShrink: 0,
                          }}
                        >
                          <IntegrationIcon id={entry.id} name={entry.name} type={entry.kind} size={15} />
                        </div>
                        <h3 style={{ fontSize: '13.5px', fontWeight: 600, color: '#F4F4F5', margin: 0 }}>
                          {entry.name}
                        </h3>
                      </div>
                      <p style={{ fontSize: '11.5px', color: '#A5A6B0', margin: '0 0 10px 0', lineHeight: 1.4, display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>
                        {entry.description}
                      </p>

                      {/* Capability Tags */}
                      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px', marginBottom: '10px' }}>
                        {entry.supported_capabilities.slice(0, 3).map((c) => (
                          <span
                            key={c}
                            style={{
                              fontSize: '10px',
                              fontFamily: 'var(--font-mono)',
                              color: '#9CA3AF',
                              backgroundColor: '#1E1F24',
                              padding: '2px 5px',
                              borderRadius: '3px',
                              border: '1px solid #2D2E35',
                            }}
                          >
                            {c}
                          </span>
                        ))}
                        {entry.supported_capabilities.length > 3 && (
                          <span style={{ fontSize: '10px', color: '#71717A', padding: '2px' }}>
                            +{entry.supported_capabilities.length - 3} more
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Bottom CTA */}
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'flex-end', paddingTop: '8px', borderTop: '1px solid #2D2E35' }}>
                      <span
                        style={{
                          fontSize: '11px',
                          color: '#7C7CF8',
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '4px',
                          fontWeight: 500,
                        }}
                      >
                        <span>Configure / Details</span>
                        <ArrowRight size={11} />
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
