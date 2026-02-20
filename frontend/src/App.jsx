import React, { useState, useEffect } from 'react';
import {
  Shield,
  Settings,
  BarChart3,
  Plus,
  Play,
  Zap,
  CheckCircle2,
  AlertCircle,
  Loader2,
  Trash2,
  ChevronRight,
  Globe
} from 'lucide-react';

const API_BASE = 'http://localhost:3000/api';

function App() {
  const [activeTab, setActiveTab] = useState('configurations');
  const [configs, setConfigs] = useState([]);
  const [results, setResults] = useState([]);
  const [activeJobs, setActiveJobs] = useState({}); // { job_id: { progress, current_target, state, ... } }
  const [showModal, setShowModal] = useState(false);
  const [loading, setLoading] = useState(false);

  // Form State
  const [name, setName] = useState('');
  const [urls, setUrls] = useState(['']);
  const [scanType, setScanType] = useState('passive');
  const [duration, setDuration] = useState(15);

  useEffect(() => {
    fetchConfigs();
    fetchResults();
    fetchActiveJobs();
  }, []);

  const fetchActiveJobs = async () => {
    try {
      const res = await fetch(`${API_BASE}/jobs/active`);
      const data = await res.json();
      const active = {};
      data.forEach(job => {
        active[job.job_id] = job;
        startProgressStream(job.job_id, job.config_name);
      });
      setActiveJobs(active);
    } catch (e) { console.error(e); }
  };

  const fetchConfigs = async () => {
    try {
      const res = await fetch(`${API_BASE}/configs`);
      const data = await res.json();
      setConfigs(data);
    } catch (e) { console.error(e); }
  };

  const fetchResults = async () => {
    try {
      const res = await fetch(`${API_BASE}/results`);
      const data = await res.json();
      setResults(data);
    } catch (e) { console.error(e); }
  };

  const saveConfig = async () => {
    const payload = { name, urls: urls.filter(u => u.trim()), scan_type: scanType, duration };
    try {
      await fetch(`${API_BASE}/configs`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
      fetchConfigs();
      setShowModal(false);
      setName(''); setUrls(['']);
    } catch (e) { console.error(e); }
  };

  const startProgressStream = (jobId, configName) => {
    const eventSource = new EventSource(`${API_BASE}/jobs/${jobId}/events`);

    eventSource.onmessage = (event) => {
      const data = JSON.parse(event.data);
      console.log('Progress Update:', data);

      setActiveJobs(prev => ({
        ...prev,
        [jobId]: { ...data, config_name: configName }
      }));

      if (data.state === 'completed' || data.state === 'aborted') {
        eventSource.close();
        // Remove from active after a delay and refresh results
        setTimeout(() => {
          setActiveJobs(prev => {
            const next = { ...prev };
            delete next[jobId];
            return next;
          });
          fetchResults();
        }, 3000);
      }
    };

    eventSource.onerror = (e) => {
      console.error('SSE Error:', e);
      eventSource.close();
    };
  };

  const launchScan = async (id, configName) => {
    try {
      const res = await fetch(`${API_BASE}/configs/${id}/launch`, { method: 'POST' });
      const data = await res.json();
      const jobId = data.job_id;

      setActiveJobs(prev => ({
        ...prev,
        [jobId]: { state: 'starting', config_name: configName, progress: '0/0', current_target: 'Preparing...' }
      }));

      startProgressStream(jobId, configName);
    } catch (e) { console.error(e); }
  };

  const launchAll = async () => {
    try {
      const res = await fetch(`${API_BASE}/configs/launch-all`, { method: 'POST' });
      const launched = await res.json();

      launched.forEach(job => {
        setActiveJobs(prev => ({
          ...prev,
          [job.job_id]: { state: 'starting', config_name: job.config_name, progress: '0/0', current_target: 'Preparing...' }
        }));
        startProgressStream(job.job_id, job.config_name);
      });
    } catch (e) { console.error(e); }
  };

  return (
    <div className="min-h-screen">
      {/* Header */}
      <header className="bg-white border-b border-gray-200 py-4 px-8 flex justify-between items-center sticky top-0 z-10">
        <div className="flex items-center gap-3">
          <div className="bg-[#00897b] p-2 rounded-lg">
            <Shield className="text-white" size={24} />
          </div>
          <h1 className="text-xl font-bold tracking-tight text-[#005b4f]">BLUE WEB COMPANY</h1>
        </div>
        <nav className="flex gap-1 bg-gray-100 p-1 rounded-xl">
          <button
            onClick={() => setActiveTab('configurations')}
            className={`btn gap-2 px-6 ${activeTab === 'configurations' ? 'bg-white shadow-sm text-[#00897b]' : 'text-gray-500'}`}
          >
            <Settings size={18} /> Configurations
          </button>
          <button
            onClick={() => setActiveTab('results')}
            className={`btn gap-2 px-6 ${activeTab === 'results' ? 'bg-white shadow-sm text-[#00897b]' : 'text-gray-500'}`}
          >
            <BarChart3 size={18} /> Results
          </button>
        </nav>
      </header>

      <main className="max-w-7xl mx-auto p-8">
        {activeTab === 'configurations' ? (
          <div className="space-y-6">
            <div className="card bg-blue-50 border-blue-200 flex gap-4 items-start">
              <div className="bg-blue-500 p-2 rounded-full mt-1">
                <AlertCircle className="text-white" size={20} />
              </div>
              <div>
                <h3 className="font-semibold text-blue-900">Web Application Scanner</h3>
                <p className="text-blue-700 text-sm mt-1 leading-relaxed">
                  Identify and assess vulnerabilities within web applications that could be exploited by attackers.
                  This tool assists in prioritizing security measures and maintaining compliance with security standards.
                </p>
              </div>
            </div>

            {/* Active Scans Section */}
            {Object.keys(activeJobs).length > 0 && (
              <div className="space-y-4">
                <h2 className="text-xl font-bold flex items-center gap-2">
                  <Loader2 className="animate-spin text-[#00897b]" size={20} />
                  Active Scans
                </h2>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  {Object.entries(activeJobs).map(([id, job]) => (
                    <div key={id} className="card border-l-4 border-l-[#00897b] flex flex-col gap-3">
                      <div className="flex justify-between items-start">
                        <div>
                          <h4 className="font-bold text-gray-900">{job.config_name}</h4>
                          <p className="text-xs text-gray-500 font-mono mt-1">{id}</p>
                        </div>
                        <span className={`badge ${job.state === 'starting' ? 'badge-info' : 'badge-warning'} animate-pulse`}>
                          {job.state.toUpperCase()}
                        </span>
                      </div>

                      <div className="space-y-2">
                        <div className="flex justify-between text-xs font-medium">
                          <span className="text-gray-600">Progress: {job.progress}</span>
                          <span className="text-[#00897b] truncate ml-4 max-w-[200px]">{job.current_target}</span>
                        </div>
                        <div className="w-full bg-gray-200 rounded-full h-1.5 overflow-hidden">
                          <div
                            className="bg-[#00897b] h-full transition-all duration-500"
                            style={{
                              width: job.progress.includes('/')
                                ? `${(parseInt(job.progress.split('/')[0]) / parseInt(job.progress.split('/')[1])) * 100}%`
                                : '10%'
                            }}
                          ></div>
                        </div>
                      </div>

                      {job.state === 'completed' && (
                        <div className="text-xs text-green-600 font-bold flex items-center gap-1 mt-1">
                          <CheckCircle2 size={12} /> Scan Complete. Syncing results...
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div className="flex justify-between items-center">
              <h2 className="text-2xl font-bold">URL Configurations <span className="text-sm bg-gray-100 px-2 py-1 rounded-full ml-2 text-gray-500">{configs.length}</span></h2>
              <div className="flex gap-3">
                <button onClick={launchAll} className="btn bg-[#00897b] text-white">
                  <Play size={18} /> Launch All
                </button>
                <button onClick={() => setShowModal(true)} className="btn bg-white border border-gray-300">
                  <Plus size={18} /> Add Config
                </button>
              </div>
            </div>

            <div className="table-container shadow-sm bg-white">
              <table>
                <thead>
                  <tr>
                    <th>NAME</th>
                    <th>URLS</th>
                    <th>SCAN TYPE</th>
                    <th>DURATION</th>
                    <th>ACTION</th>
                  </tr>
                </thead>
                <tbody>
                  {configs.map(config => (
                    <tr key={config.id}>
                      <td className="font-medium">{config.name}</td>
                      <td className="text-gray-500">
                        {JSON.parse(config.urls).length > 1
                          ? `${JSON.parse(config.urls).length} URLs`
                          : JSON.parse(config.urls)[0]}
                      </td>
                      <td>
                        <span className={`badge ${config.scan_type === 'active' ? 'badge-danger' : 'badge-info'}`}>
                          {config.scan_type}
                        </span>
                      </td>
                      <td>{config.duration} min</td>
                      <td>
                        <button onClick={() => launchScan(config.id, config.name)} className="text-[#00897b] hover:text-[#005b4f]">
                          <Play size={18} title="Launch" />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        ) : (
          <div className="space-y-6">
            <div className="flex justify-between items-center">
              <h2 className="text-2xl font-bold">Scan Results</h2>
              <button onClick={fetchResults} className="btn btn-outline">Refresh</button>
            </div>

            <div className="table-container shadow-sm bg-white">
              <table>
                <thead>
                  <tr>
                    <th>JOB NAME</th>
                    <th>URL</th>
                    <th>DATE</th>
                    <th>VULNERABILITIES</th>
                    <th>SEVERITY BREAKDOWN</th>
                    <th>ACTION</th>
                  </tr>
                </thead>
                <tbody>
                  {results.map(res => (
                    <tr key={res.id}>
                      <td className="font-semibold text-gray-900">{res.config_name}</td>
                      <td className="font-medium">{res.url}</td>
                      <td className="text-gray-500">{new Date(res.created_at * 1000).toLocaleString()}</td>
                      <td>
                        <span className="font-bold text-lg">{res.total_vulnerabilities}</span>
                      </td>
                      <td>
                        <div className="flex gap-2">
                          <span className="badge badge-danger" title="High">{res.high_sev} H</span>
                          <span className="badge badge-warning" title="Medium">{res.medium_sev} M</span>
                          <span className="badge badge-info" title="Low">{res.low_sev} L</span>
                        </div>
                      </td>
                      <td>
                        <a
                          href={`http://localhost:3000/reports/${res.report_path.split('/').pop()}`}
                          target="_blank"
                          rel="noreferrer"
                          className="text-[#00897b] font-semibold hover:underline"
                        >
                          View Detail
                        </a>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </main>

      {/* Add Config Modal */}
      {showModal && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 z-50">
          <div className="card w-full max-w-lg shadow-2xl animate-in fade-in zoom-in duration-200">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-xl font-bold">New Configuration</h2>
              <button onClick={() => setShowModal(false)} className="text-gray-400 hover:text-gray-600">✕</button>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Name</label>
                <input
                  value={name} onChange={e => setName(e.target.value)}
                  className="w-full p-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#00897b] outline-none"
                  placeholder="e.g. My Website Scan"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">URLs</label>
                {urls.map((url, i) => (
                  <div key={i} className="flex gap-2 mb-2">
                    <input
                      value={url} onChange={e => {
                        const next = [...urls]; next[i] = e.target.value; setUrls(next);
                      }}
                      className="flex-1 p-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#00897b] outline-none"
                      placeholder="https://example.com"
                    />
                    {urls.length > 1 && (
                      <button onClick={() => setUrls(urls.filter((_, idx) => idx !== i))} className="p-2 text-red-500">
                        <Trash2 size={18} />
                      </button>
                    )}
                  </div>
                ))}
                <button onClick={() => setUrls([...urls, ''])} className="text-sm text-[#00897b] font-medium">+ Add URL</button>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Scan Type</label>
                  <select
                    value={scanType} onChange={e => setScanType(e.target.value)}
                    className="w-full p-2 border border-gray-300 rounded-lg outline-none"
                  >
                    <option value="passive">Passive</option>
                    <option value="active">Active</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Duration (Min)</label>
                  <input
                    type="number" value={duration} onChange={e => setDuration(e.target.value)}
                    className="w-full p-2 border border-gray-300 rounded-lg outline-none"
                  />
                </div>
              </div>

              <div className="flex gap-3 pt-6">
                <button onClick={() => setShowModal(false)} className="btn btn-outline flex-1">Cancel</button>
                <button onClick={saveConfig} className="btn bg-[#00897b] text-white flex-1">Save Configuration</button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
