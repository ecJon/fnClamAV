const { createApp } = Vue;

createApp({
    data() {
        return {
            // API 基础路径 (通过 CGI 转发到 Rust daemon)
            apiBase: './api/',

            // 当前激活的标签页
            activeTab: 'threats',

            // 标签页配置
            tabs: [
                { key: 'threats', label: '威胁列表' },
                { key: 'quarantine', label: '隔离区' },
                { key: 'history', label: '扫描历史' },
                { key: 'updates', label: '病毒库更新' },
                { key: 'settings', label: '设置' }
            ],

            // 系统状态
            systemStatus: {
                is_scanning: false
            },

            // 扫描状态
            scanStatus: {
                scan_id: null,
                status: 'idle',      // idle, scanning, completed, failed
                progress: null,
                threats: null
            },

            // UI 显示状态
            uiState: {
                showProgress: false  // 是否显示进度卡片
            },

            // 更新状态
            updateStatus: {
                is_updating: false
            },

            // 病毒库版本
            virusVersion: {
                version: null,
                date: null
            },

            // 威胁列表
            threats: [],

            // 隔离区列表
            quarantineList: [],

            // 扫描历史
            scanHistory: [],

            // 更新历史
            updateHistory: [],

            // 配置
            config: {
                scan_paths: '',
                auto_update: true,
                quarantine_enabled: true,
                threat_action: 'quarantine'
            },

            // 通知
            notification: {
                show: false,
                message: '',
                type: 'info',
                timeout: null
            },

            // 轮询定时器
            pollTimer: null
        };
    },

    computed: {
        totalThreats() {
            return this.scanHistory.reduce((sum, scan) => sum + (scan.threats_found || 0), 0);
        }
    },

    mounted() {
        this.loadInitialData();
        this.startPolling();
    },

    beforeUnmount() {
        this.stopPolling();
    },

    methods: {
        // API 请求封装
        async apiRequest(endpoint, options = {}) {
            try {
                const url = this.apiBase + endpoint;
                const response = await fetch(url, {
                    headers: {
                        'Content-Type': 'application/json',
                        ...options.headers
                    },
                    ...options
                });

                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }

                return await response.json();
            } catch (error) {
                console.error('API request failed:', error);
                this.showNotification(error.message, 'error');
                throw error;
            }
        },

        // 加载初始数据
        async loadInitialData() {
            await Promise.all([
                this.loadSystemStatus(),
                this.loadVirusVersion(),
                this.loadThreats(),
                this.loadQuarantine(),
                this.loadScanHistory(),
                this.loadUpdateHistory(),
                this.loadConfig()
            ]);
        },

        // 加载系统状态
        async loadSystemStatus() {
            try {
                const data = await this.apiRequest('status');
                this.systemStatus.is_scanning = data.scan_in_progress || false;
            } catch (error) {
                console.error('Failed to load system status:', error);
            }
        },

        // 加载扫描状态
        async loadScanStatus() {
            try {
                const data = await this.apiRequest('scan/status');
                const oldStatus = this.scanStatus.status;
                const newStatus = data.status;

                // 更新扫描状态
                this.scanStatus = {
                    scan_id: data.scan_id,
                    status: newStatus,
                    progress: data.progress,
                    threats: data.threats
                };

                // 状态转换逻辑
                if (newStatus === 'scanning' && !this.uiState.showProgress) {
                    // 开始扫描 -> 显示进度
                    this.uiState.showProgress = true;
                } else if ((newStatus === 'completed' || newStatus === 'failed') && oldStatus === 'scanning') {
                    // 扫描刚完成 -> 显示完成通知，保持显示进度 5 秒后隐藏
                    if (newStatus === 'completed') {
                        const threats = data.threats?.count || 0;
                        if (threats > 0) {
                            this.showNotification(`扫描完成，发现 ${threats} 个威胁`, 'warning');
                        } else {
                            this.showNotification('扫描完成，未发现威胁', 'success');
                        }
                    } else {
                        this.showNotification('扫描失败', 'error');
                    }

                    setTimeout(() => {
                        this.uiState.showProgress = false;
                        this.scanStatus.status = 'idle';
                        this.scanStatus.progress = null;
                        this.loadScanHistory(); // 刷新历史记录
                    }, 5000);
                }
            } catch (error) {
                console.error('Failed to load scan status:', error);
            }
        },

        // 加载病毒库版本
        async loadVirusVersion() {
            try {
                const data = await this.apiRequest('update/version');
                const versionInfo = data.version || {};

                // 提取实际版本号（去掉 "days old" 后缀）
                const extractVersion = (v) => {
                    if (!v || v === '未知') return null;
                    return v.replace(/\s*days\s*old.*/gi, '').trim() || null;
                };

                const daily = extractVersion(versionInfo.daily);
                const main = extractVersion(versionInfo.main);
                const bytecode = extractVersion(versionInfo.bytecode);

                // 格式化版本显示
                this.virusVersion = {
                    daily: daily || '未知',
                    main: main || '未知',
                    bytecode: bytecode || '未知',
                    // 兼容旧的显示方式
                    version: daily ? `Daily ${daily}` : '未知',
                    date: main ? `Main ${main}` : '-'
                };
            } catch (error) {
                console.error('Failed to load virus version:', error);
                this.virusVersion = { daily: '未知', main: '未知', bytecode: '未知', version: '未知', date: '-' };
            }
        },

        // 加载威胁列表
        async loadThreats() {
            try {
                const data = await this.apiRequest('threats');
                this.threats = data.threats || [];
            } catch (error) {
                console.error('Failed to load threats:', error);
                this.threats = [];
            }
        },

        // 加载隔离区
        async loadQuarantine() {
            try {
                const data = await this.apiRequest('quarantine');
                this.quarantineList = data.items || [];
            } catch (error) {
                console.error('Failed to load quarantine:', error);
                this.quarantineList = [];
            }
        },

        // 加载扫描历史
        async loadScanHistory() {
            try {
                const data = await this.apiRequest('scan/history');
                this.scanHistory = data.items || [];
            } catch (error) {
                console.error('Failed to load scan history:', error);
                this.scanHistory = [];
            }
        },

        // 加载更新历史
        async loadUpdateHistory() {
            try {
                const data = await this.apiRequest('update/history');
                this.updateHistory = data.items || [];
            } catch (error) {
                console.error('Failed to load update history:', error);
                this.updateHistory = [];
            }
        },

        // 加载配置
        async loadConfig() {
            try {
                const data = await this.apiRequest('config');
                if (data.scan_paths) {
                    this.config = {
                        scan_paths: Array.isArray(data.scan_paths) ? data.scan_paths.join('\n') : data.scan_paths,
                        auto_update: data.auto_update ?? true,
                        quarantine_enabled: data.quarantine_enabled ?? true,
                        threat_action: data.threat_action || 'quarantine'
                    };
                }
            } catch (error) {
                console.error('Failed to load config:', error);
            }
        },

        // 开始全盘扫描
        async startFullScan() {
            try {
                const result = await this.apiRequest('scan/start', {
                    method: 'POST',
                    body: JSON.stringify({
                        scan_type: 'full'
                    })
                });

                if (result.success) {
                    this.showNotification('全盘扫描已启动', 'success');
                    this.scanStatus.scan_id = result.scan_id;
                    this.scanStatus.status = 'scanning';
                    this.uiState.showProgress = true;  // 立即显示进度条
                    await this.loadSystemStatus();
                    await this.loadScanHistory();
                } else {
                    this.showNotification(result.error || '启动扫描失败', 'error');
                }
            } catch (error) {
                this.showNotification('启动扫描失败: ' + error.message, 'error');
            }
        },

        // 开始自定义扫描
        async startCustomScan() {
            try {
                // 从配置中获取扫描路径
                const paths = this.config.scan_paths.split('\n')
                    .map(p => p.trim())
                    .filter(p => p.length > 0);

                if (paths.length === 0) {
                    this.showNotification('请先在设置中配置扫描路径', 'warning');
                    this.activeTab = 'settings';
                    return;
                }

                const result = await this.apiRequest('scan/start', {
                    method: 'POST',
                    body: JSON.stringify({
                        scan_type: 'custom',
                        paths: paths
                    })
                });

                if (result.success) {
                    this.showNotification('自定义扫描已启动', 'success');
                    this.scanStatus.scan_id = result.scan_id;
                    this.scanStatus.status = 'scanning';
                    this.uiState.showProgress = true;  // 立即显示进度条
                    await this.loadSystemStatus();
                    await this.loadScanHistory();
                } else {
                    this.showNotification(result.error || '启动扫描失败', 'error');
                }
            } catch (error) {
                this.showNotification('启动扫描失败: ' + error.message, 'error');
            }
        },

        // 停止扫描
        async stopScan() {
            try {
                const result = await this.apiRequest('scan/stop', {
                    method: 'POST'
                });

                if (result.success) {
                    this.showNotification('扫描已停止', 'info');
                    await this.loadSystemStatus();
                    await this.loadScanHistory();
                } else {
                    this.showNotification(result.error || '停止扫描失败', 'error');
                }
            } catch (error) {
                this.showNotification('停止扫描失败: ' + error.message, 'error');
            }
        },

        // 开始更新病毒库
        async startUpdate() {
            try {
                const result = await this.apiRequest('update/start', {
                    method: 'POST',
                    body: JSON.stringify({})
                });

                if (result.success) {
                    this.showNotification('病毒库更新已启动', 'success');
                    this.updateStatus.is_updating = true;
                } else {
                    this.showNotification(result.error || '启动更新失败', 'error');
                }
            } catch (error) {
                this.showNotification('启动更新失败: ' + error.message, 'error');
            }
        },

        // 处理威胁
        async handleThreat(threatId, action) {
            try {
                const result = await this.apiRequest(`threats/${threatId}/handle`, {
                    method: 'POST',
                    body: JSON.stringify({ action })
                });

                if (result.success) {
                    this.showNotification('威胁已处理', 'success');
                    await this.loadThreats();
                } else {
                    this.showNotification(result.error || '处理威胁失败', 'error');
                }
            } catch (error) {
                this.showNotification('处理威胁失败: ' + error.message, 'error');
            }
        },

        // 恢复隔离文件
        async restoreQuarantine(uuid) {
            try {
                const result = await this.apiRequest(`quarantine/${uuid}/restore`, {
                    method: 'POST'
                });

                if (result.success) {
                    this.showNotification('文件已恢复', 'success');
                    await this.loadQuarantine();
                } else {
                    this.showNotification(result.error || '恢复文件失败', 'error');
                }
            } catch (error) {
                this.showNotification('恢复文件失败: ' + error.message, 'error');
            }
        },

        // 删除隔离文件
        async deleteQuarantine(uuid) {
            if (!confirm('确定要永久删除此文件吗？')) {
                return;
            }

            try {
                const result = await this.apiRequest(`quarantine/${uuid}`, {
                    method: 'DELETE'
                });

                if (result.success) {
                    this.showNotification('文件已删除', 'success');
                    await this.loadQuarantine();
                } else {
                    this.showNotification(result.error || '删除文件失败', 'error');
                }
            } catch (error) {
                this.showNotification('删除文件失败: ' + error.message, 'error');
            }
        },

        // 清理隔离区
        async cleanupQuarantine() {
            if (!confirm('确定要清理隔离区吗？这将删除所有隔离文件。')) {
                return;
            }

            try {
                const result = await this.apiRequest('quarantine/cleanup', {
                    method: 'POST'
                });

                if (result.success) {
                    this.showNotification('隔离区已清理', 'success');
                    await this.loadQuarantine();
                } else {
                    this.showNotification(result.error || '清理失败', 'error');
                }
            } catch (error) {
                this.showNotification('清理失败: ' + error.message, 'error');
            }
        },

        // 保存配置
        async saveConfig() {
            try {
                const paths = this.config.scan_paths.split('\n').filter(p => p.trim());
                const result = await this.apiRequest('config', {
                    method: 'PUT',
                    body: JSON.stringify({
                        scan_paths: paths,
                        auto_update: this.config.auto_update,
                        quarantine_enabled: this.config.quarantine_enabled,
                        threat_action: this.config.threat_action
                    })
                });

                if (result.success) {
                    this.showNotification('配置已保存', 'success');
                } else {
                    this.showNotification(result.error || '保存配置失败', 'error');
                }
            } catch (error) {
                this.showNotification('保存配置失败: ' + error.message, 'error');
            }
        },

        // 显示通知
        showNotification(message, type = 'info') {
            // 清除之前的定时器
            if (this.notification.timeout) {
                clearTimeout(this.notification.timeout);
            }

            this.notification = {
                show: true,
                message,
                type
            };

            // 3秒后自动隐藏
            this.notification.timeout = setTimeout(() => {
                this.notification.show = false;
            }, 3000);
        },

        // 截断过长的文件路径
        truncatePath(path) {
            if (!path) return '';
            const maxLength = 60;
            if (path.length <= maxLength) return path;

            // 保留开头和结尾，中间用...代替
            const startLength = Math.floor(maxLength / 2) - 2;
            const endLength = Math.floor(maxLength / 2) - 1;
            return path.substring(0, startLength) + '...' + path.substring(path.length - endLength);
        },

        // 开始轮询状态
        startPolling() {
            this.pollTimer = setInterval(async () => {
                // 总是获取系统状态
                await this.loadSystemStatus();

                // 只在有扫描进行或显示进度时获取扫描状态
                if (this.systemStatus.is_scanning || this.uiState.showProgress) {
                    await this.loadScanStatus();
                }

                // 处理病毒库更新状态
                if (this.updateStatus.is_updating) {
                    const data = await this.apiRequest('update/status');
                    this.updateStatus.is_updating = data.is_updating || false;
                    if (!this.updateStatus.is_updating) {
                        await this.loadVirusVersion();
                        await this.loadUpdateHistory();
                    }
                }
            }, 2000);
        },

        // 停止轮询
        stopPolling() {
            if (this.pollTimer) {
                clearInterval(this.pollTimer);
                this.pollTimer = null;
            }
        }
    }
}).mount('#app');
