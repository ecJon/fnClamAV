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

            // 连接状态
            connectionStatus: {
                connected: false,      // 是否已连接到 daemon
                checking: true,        // 是否正在检查连接
                lastCheck: null,       // 上次检查时间
                retryCount: 0          // 重试次数
            },

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
        },

        // 界面是否可用（取决于连接状态）
        isReady() {
            return this.connectionStatus.connected;
        }
    },

    mounted() {
        // 首先检查连接状态
        this.checkConnection();
    },

    beforeUnmount() {
        this.stopPolling();
    },

    methods: {
        // API 请求封装（带连接状态检测）
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

                const data = await response.json();

                // 检查是否有连接错误
                if (data.success === false && data.error === 'Failed to connect to daemon') {
                    this.handleConnectionLost();
                    throw new Error('无法连接到服务');
                }

                return data;
            } catch (error) {
                console.error('API request failed:', error);
                throw error;
            }
        },

        // 检查与 daemon 的连接状态
        async checkConnection() {
            this.connectionStatus.checking = true;

            try {
                const response = await fetch(this.apiBase + 'status');
                if (!response.ok) {
                    throw new Error('Connection failed');
                }

                const data = await response.json();

                // 检查是否是有效的 daemon 响应
                if (data.success === false && data.error === 'Failed to connect to daemon') {
                    this.handleConnectionLost();
                    return false;
                }

                // 连接成功
                this.handleConnectionRestored(data);
                return true;
            } catch (error) {
                this.handleConnectionLost();
                return false;
            } finally {
                this.connectionStatus.checking = false;
            }
        },

        // 处理连接丢失
        handleConnectionLost() {
            const wasConnected = this.connectionStatus.connected;
            this.connectionStatus.connected = false;
            this.connectionStatus.retryCount++;
            this.connectionStatus.lastCheck = new Date();

            if (wasConnected) {
                this.showNotification('与服务失去连接，正在重连...', 'error');
            }

            // 如果未连接，每 2 秒重试一次
            if (!this.connectionStatus.connected) {
                setTimeout(() => {
                    this.checkConnection();
                }, 2000);
            }
        },

        // 处理连接恢复
        handleConnectionRestored(data) {
            const wasDisconnected = !this.connectionStatus.connected;
            this.connectionStatus.connected = true;
            this.connectionStatus.retryCount = 0;
            this.connectionStatus.lastCheck = new Date();

            // 更新系统状态
            if (data) {
                this.systemStatus.is_scanning = data.scan_in_progress || false;
            }

            if (wasDisconnected) {
                this.showNotification('已连接到服务', 'success');
                // 连接恢复后加载数据
                this.loadInitialData();
                // 开始常规轮询
                this.startPolling();
            }
        },

        // 加载初始数据
        async loadInitialData() {
            if (!this.connectionStatus.connected) {
                return;
            }

            await Promise.all([
                this.loadVirusVersion(),
                this.loadThreats(),
                this.loadQuarantine(),
                this.loadScanHistory(),
                this.loadUpdateHistory(),
                this.loadConfig()
            ]);
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
                        this.loadThreats(); // 刷新威胁列表
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
                this.threats = data.items || [];
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

        // 删除单条扫描历史
        async deleteScanHistory(id) {
            try {
                const result = await this.apiRequest(`scan/history/${id}`, {
                    method: 'DELETE'
                });

                if (result.success) {
                    this.showNotification('记录已删除', 'success');
                    await this.loadScanHistory();
                } else {
                    this.showNotification(result.error || '删除失败', 'error');
                }
            } catch (error) {
                this.showNotification('删除失败: ' + error.message, 'error');
            }
        },

        // 清空所有扫描历史
        async clearScanHistory() {
            if (!confirm('确定要清空所有扫描历史记录吗？')) {
                return;
            }

            try {
                const result = await this.apiRequest('scan/history/clear', {
                    method: 'POST'
                });

                if (result.success) {
                    this.showNotification('扫描历史已清空', 'success');
                    await this.loadScanHistory();
                } else {
                    this.showNotification(result.error || '清空失败', 'error');
                }
            } catch (error) {
                this.showNotification('清空失败: ' + error.message, 'error');
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
                    this.systemStatus.is_scanning = true;
                    this.uiState.showProgress = true;  // 立即显示进度条
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
                    this.systemStatus.is_scanning = true;
                    this.uiState.showProgress = true;  // 立即显示进度条
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
                    this.systemStatus.is_scanning = false;
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
            // 避免重复启动
            if (this.pollTimer) {
                return;
            }

            this.pollTimer = setInterval(async () => {
                // 检查连接状态
                if (!this.connectionStatus.connected) {
                    return;
                }

                try {
                    // 获取系统状态并检测连接
                    const data = await this.apiRequest('status');
                    if (data.success === false && data.error === 'Failed to connect to daemon') {
                        return; // 连接检测会处理
                    }
                    this.systemStatus.is_scanning = data.scan_in_progress || false;

                    // 只在有扫描进行或显示进度时获取扫描状态
                    if (this.systemStatus.is_scanning || this.uiState.showProgress) {
                        await this.loadScanStatus();
                    }

                    // 处理病毒库更新状态
                    if (this.updateStatus.is_updating) {
                        const updateData = await this.apiRequest('update/status');
                        this.updateStatus.is_updating = updateData.is_updating || false;
                        if (!this.updateStatus.is_updating) {
                            await this.loadVirusVersion();
                            await this.loadUpdateHistory();
                        }
                    }
                } catch (error) {
                    // 轮询中的错误不显示通知，避免刷屏
                    console.error('Polling error:', error);
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
