// ==UserScript==
// @name         EhArchive Script
// @namespace    https://github.com/AyaseFile/EhArchive
// @version      0.1.2
// @description  嵌入 E-Hentai, 配合后端使用
// @author       Ayase
// @match        *://e-hentai.org/*
// @match        *://exhentai.org/*
// @grant        GM_setValue
// @grant        GM_getValue
// @grant        GM_xmlhttpRequest
// @updateURL    https://raw.githubusercontent.com/AyaseFile/EhArchive/main/tampermonkey.user.js
// @downloadURL  https://raw.githubusercontent.com/AyaseFile/EhArchive/main/tampermonkey.user.js
// @supportURL   https://github.com/AyaseFile/EhArchive/issues
// ==/UserScript==

(function () {
    'use strict';

    let isOriginal = GM_getValue('isOriginal', false);
    let backendUrl = GM_getValue('backendUrl', 'http://localhost:3000');
    let minimized = GM_getValue('minimized', false);
    let activeTasks = [];
    let updateTasksInterval = null;

    let colorScheme = {
        background: '#2d2d2d',
        text: '#ffffff',
        border: '#555555',
        buttonBg: '#444444',
        buttonText: '#ffffff',
        buttonHover: '#555555',
        inputBg: '#3d3d3d',
        inputBorder: '#666666',
        shadow: 'rgba(0,0,0,0.3)',
        taskBg: '#3a3a3a',
        bubbleBg: '#4a4a4a',
        bubbleBorder: '#666666'
    };

    const container = document.createElement('div');
    container.style.position = 'fixed';
    container.style.bottom = '20px';
    container.style.left = '20px';
    container.style.padding = '8px';
    container.style.paddingTop = '0px';
    container.style.borderRadius = '8px';
    container.style.zIndex = '9999';
    container.style.fontFamily = 'Arial, sans-serif';
    container.style.fontSize = '14px';
    container.style.lineHeight = '1.4';
    container.style.width = minimized ? 'auto' : '200px';
    container.style.transition = 'all 0.3s ease';

    const taskBubble = document.createElement('div');
    taskBubble.style.position = 'absolute';
    taskBubble.style.top = '-38px';
    taskBubble.style.left = '-14px';
    taskBubble.style.backgroundColor = colorScheme.bubbleBg;
    taskBubble.style.border = `1px solid ${colorScheme.bubbleBorder}`;
    taskBubble.style.borderRadius = '4px';
    taskBubble.style.padding = '6px 10px';
    taskBubble.style.fontSize = '12px';
    taskBubble.style.color = colorScheme.text;
    taskBubble.style.boxShadow = `0 0 5px ${colorScheme.shadow}`;
    taskBubble.style.display = 'none';
    taskBubble.style.zIndex = '10000';
    taskBubble.style.whiteSpace = 'nowrap';
    taskBubble.style.cursor = 'pointer';

    const bubbleArrow = document.createElement('div');
    bubbleArrow.style.position = 'absolute';
    bubbleArrow.style.bottom = '-5px';
    bubbleArrow.style.left = '20px';
    bubbleArrow.style.width = '10px';
    bubbleArrow.style.height = '10px';
    bubbleArrow.style.backgroundColor = colorScheme.bubbleBg;
    bubbleArrow.style.border = `1px solid ${colorScheme.bubbleBorder}`;
    bubbleArrow.style.borderTop = 'none';
    bubbleArrow.style.borderLeft = 'none';
    bubbleArrow.style.transform = 'rotate(45deg)';
    taskBubble.appendChild(bubbleArrow);

    const titleDiv = document.createElement('div');
    titleDiv.textContent = '下载设置';
    titleDiv.style.fontWeight = 'bold';
    titleDiv.style.marginTop = minimized ? '0px' : '3px';
    titleDiv.style.marginBottom = '6px';
    titleDiv.style.fontSize = '14px';
    titleDiv.style.display = minimized ? 'none' : 'block';
    titleDiv.style.marginLeft = '18px';
    container.appendChild(titleDiv);

    const formContainer = document.createElement('div');
    formContainer.style.display = minimized ? 'none' : 'flex';
    formContainer.style.flexDirection = 'column';
    formContainer.style.gap = '4px';
    formContainer.style.padding = '0 4px';

    const originalFileDiv = document.createElement('div');
    originalFileDiv.style.display = 'flex';
    originalFileDiv.style.alignItems = 'center';
    originalFileDiv.style.textAlign = 'left';

    const originalFileLabel = document.createElement('label');
    originalFileLabel.textContent = '原始档: ';
    originalFileLabel.style.marginRight = '8px';
    originalFileLabel.style.display = 'flex';
    originalFileLabel.style.alignItems = 'center';
    originalFileLabel.style.height = '20px';
    originalFileLabel.style.fontSize = '14px';

    const originalFileCheckbox = document.createElement('input');
    originalFileCheckbox.type = 'checkbox';
    originalFileCheckbox.checked = isOriginal;
    originalFileCheckbox.style.cursor = 'pointer';
    originalFileCheckbox.style.margin = '0';
    originalFileCheckbox.style.marginBottom = '5px';
    originalFileCheckbox.addEventListener('change', function () {
        isOriginal = this.checked;
        GM_setValue('isOriginal', isOriginal);
    });

    originalFileDiv.appendChild(originalFileLabel);
    originalFileDiv.appendChild(originalFileCheckbox);
    formContainer.appendChild(originalFileDiv);

    const backendUrlDiv = document.createElement('div');
    backendUrlDiv.style.display = 'flex';
    backendUrlDiv.style.flexDirection = 'column';
    backendUrlDiv.style.alignItems = 'flex-start';
    backendUrlDiv.style.textAlign = 'left';

    const backendUrlLabel = document.createElement('label');
    backendUrlLabel.textContent = '后端 URL:';
    backendUrlLabel.style.marginBottom = '3px';
    backendUrlLabel.style.fontSize = '14px';

    const backendUrlInput = document.createElement('input');
    backendUrlInput.type = 'text';
    backendUrlInput.value = backendUrl;
    backendUrlInput.style.width = '100%';
    backendUrlInput.style.padding = '4px';
    backendUrlInput.style.boxSizing = 'border-box';
    backendUrlInput.style.borderRadius = '4px';
    backendUrlInput.style.fontSize = '14px';
    backendUrlInput.addEventListener('change', function () {
        backendUrl = this.value;
        GM_setValue('backendUrl', backendUrl);
        updateActiveTasks();
    });

    backendUrlDiv.appendChild(backendUrlLabel);
    backendUrlDiv.appendChild(backendUrlInput);
    formContainer.appendChild(backendUrlDiv);

    const tasksContainer = document.createElement('div');
    tasksContainer.style.marginTop = '0px';
    tasksContainer.style.display = 'none';
    tasksContainer.style.maxHeight = '100px';
    tasksContainer.style.overflowY = 'auto';
    tasksContainer.style.fontSize = '12px';
    tasksContainer.style.borderRadius = '4px';
    tasksContainer.style.padding = '0';

    formContainer.appendChild(tasksContainer);

    const downloadButton = document.createElement('button');
    downloadButton.textContent = '下载';
    downloadButton.style.padding = '6px 10px';
    downloadButton.style.marginTop = minimized ? '8px' : '6px';
    downloadButton.style.fontSize = '14px';
    downloadButton.style.cursor = 'pointer';
    downloadButton.style.borderRadius = '4px';
    downloadButton.style.fontWeight = 'bold';
    downloadButton.style.width = minimized ? '60px' : '100%';
    downloadButton.style.border = 'none';
    downloadButton.style.transition = 'all 0.2s ease';
    downloadButton.addEventListener('click', function () {
        sendDownloadRequest();
    });

    downloadButton.addEventListener('mouseover', function () {
        this.style.backgroundColor = colorScheme.buttonHover;
    });
    downloadButton.addEventListener('mouseout', function () {
        this.style.backgroundColor = colorScheme.buttonBg;
    });

    if (minimized) {
        container.appendChild(downloadButton);
    } else {
        formContainer.appendChild(downloadButton);
    }
    container.appendChild(formContainer);

    function sendDownloadRequest() {
        const currentUrl = window.location.href;

        GM_xmlhttpRequest({
            method: 'POST',
            url: `${backendUrl}/download`,
            headers: {
                'Content-Type': 'application/json'
            },
            data: JSON.stringify({
                url: currentUrl,
                download_type: isOriginal ? 'original' : 'resample'
            }),
            onload: function (response) {
                showNotification('下载请求已发送!', 'success');
                setTimeout(updateActiveTasks, 500);
            },
            onerror: function (error) {
                showNotification('发送下载请求时出错!', 'error');
            }
        });
    }

    function updateActiveTasks() {
        GM_xmlhttpRequest({
            method: 'GET',
            url: `${backendUrl}/tasks`,
            headers: {
                'Content-Type': 'application/json'
            },
            onload: function (response) {
                try {
                    const data = JSON.parse(response.responseText);
                    activeTasks = data.tasks || [];
                    updateTasksDisplay();
                    updateBubble();
                } catch (e) {
                    console.error('解析任务列表失败');
                }
            },
            onerror: function (error) {
                console.error('获取任务列表失败');
            }
        });
    }

    function updateBubble() {
        if (activeTasks.length === 0 || !minimized) {
            taskBubble.style.display = 'none';
            return;
        }

        taskBubble.style.display = 'block';
        taskBubble.textContent = `${activeTasks.length} 个下载任务进行中`;

        taskBubble.appendChild(bubbleArrow);

        taskBubble.onclick = function () {
            if (minimized) {
                minimized = false;
                GM_setValue('minimized', false);
                toggleButton.textContent = '−';
                formContainer.style.display = 'flex';
                titleDiv.style.display = 'block';
                container.style.width = '200px';
                downloadButton.style.width = '100%';
                formContainer.appendChild(downloadButton);
                updateTasksDisplay();
            }
        };
    }

    function updateTasksDisplay() {
        tasksContainer.innerHTML = '';

        if (activeTasks.length === 0) {
            tasksContainer.style.display = 'none';
            return;
        }

        tasksContainer.style.display = 'block';

        const taskHeader = document.createElement('div');
        taskHeader.textContent = `活动任务 (${activeTasks.length})`;
        taskHeader.style.fontWeight = 'bold';
        taskHeader.style.marginTop = '0px';
        taskHeader.style.marginBottom = '6px';
        taskHeader.style.fontSize = '14px';
        tasksContainer.appendChild(taskHeader);

        const taskList = document.createElement('div');
        taskList.style.backgroundColor = colorScheme.taskBg;
        taskList.style.borderRadius = '4px';
        taskList.style.padding = '4px 6px';

        activeTasks.forEach(task => {
            const taskItem = document.createElement('div');

            let displayText = task;

            if (task.includes('/g/')) {
                const parts = task.split('/');
                const gIndex = parts.indexOf('g');

                if (gIndex !== -1 && gIndex + 2 < parts.length) {
                    const gid = parts[gIndex + 1];
                    const token = parts[gIndex + 2];
                    displayText = `${gid}_${token}`;
                }
            }

            taskItem.textContent = `• ${displayText}`;
            taskItem.style.overflow = 'hidden';
            taskItem.style.textOverflow = 'ellipsis';
            taskItem.style.whiteSpace = 'nowrap';
            taskItem.style.fontSize = '14px';
            taskItem.style.marginBottom = '2px';
            taskList.appendChild(taskItem);
        });

        tasksContainer.appendChild(taskList);
    }

    function showNotification(message, type) {
        const notification = document.createElement('div');
        notification.textContent = message;
        notification.style.position = 'fixed';

        const containerRect = container.getBoundingClientRect();
        notification.style.bottom = `${window.innerHeight - containerRect.bottom}px`;
        notification.style.left = `${containerRect.right + 10}px`;
        notification.style.padding = '8px 12px';
        notification.style.borderRadius = '4px';
        notification.style.zIndex = '10000';
        notification.style.color = '#fff';
        notification.style.backgroundColor = type === 'success' ? '#4CAF50' : '#F44336';
        notification.style.boxShadow = '0 2px 5px rgba(0,0,0,0.2)';
        notification.style.opacity = '0';
        notification.style.transition = 'opacity 0.3s ease';
        notification.style.fontSize = '13px';

        document.body.appendChild(notification);

        setTimeout(() => {
            notification.style.opacity = '1';
        }, 10);

        setTimeout(() => {
            notification.style.opacity = '0';
            setTimeout(() => {
                document.body.removeChild(notification);
            }, 300);
        }, 3000);
    }

    const toggleButton = document.createElement('button');
    toggleButton.textContent = minimized ? '⚙️' : '−';
    toggleButton.style.position = 'absolute';
    toggleButton.style.top = '4px';
    toggleButton.style.left = '4px';
    toggleButton.style.width = '18px';
    toggleButton.style.height = '18px';
    toggleButton.style.cursor = 'pointer';
    toggleButton.style.border = 'none';
    toggleButton.style.borderRadius = '50%';
    toggleButton.style.fontSize = '12px';
    toggleButton.style.display = 'flex';
    toggleButton.style.alignItems = 'center';
    toggleButton.style.justifyContent = 'center';
    toggleButton.style.padding = '0';

    toggleButton.addEventListener('click', function () {
        minimized = !minimized;
        GM_setValue('minimized', minimized);

        if (minimized) {
            toggleButton.textContent = '⚙️';
            formContainer.style.display = 'none';
            titleDiv.style.display = 'none';
            container.style.padding = '8px';
            container.style.paddingTop = '0px';
            container.style.width = 'auto';
            downloadButton.style.width = '60px';
            downloadButton.style.marginTop = '8px';
            container.appendChild(downloadButton);
        } else {
            toggleButton.textContent = '−';
            formContainer.style.display = 'flex';
            titleDiv.style.display = 'block';
            titleDiv.style.marginTop = '3px';
            container.style.padding = '8px';
            container.style.paddingTop = '0px';
            container.style.width = '200px';
            downloadButton.style.width = '100%';
            downloadButton.style.marginTop = '4px';
            formContainer.appendChild(downloadButton);
        }

        updateBubble();
    });

    container.appendChild(toggleButton);
    container.appendChild(taskBubble);

    function updateStyles() {
        container.style.backgroundColor = colorScheme.background;
        container.style.color = colorScheme.text;
        container.style.border = `1px solid ${colorScheme.border}`;
        container.style.boxShadow = `0 0 8px ${colorScheme.shadow}`;

        backendUrlInput.style.backgroundColor = colorScheme.inputBg;
        backendUrlInput.style.color = colorScheme.text;
        backendUrlInput.style.border = `1px solid ${colorScheme.inputBorder}`;

        downloadButton.style.backgroundColor = colorScheme.buttonBg;
        downloadButton.style.color = colorScheme.buttonText;

        toggleButton.style.backgroundColor = colorScheme.buttonBg;
        toggleButton.style.color = colorScheme.buttonText;
    }

    updateStyles();

    document.body.appendChild(container);

    updateActiveTasks();

    updateTasksInterval = setInterval(updateActiveTasks, 5000);
})();