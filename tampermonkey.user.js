// ==UserScript==
// @name         EhArchive Script
// @namespace    https://github.com/AyaseFile/EhArchive
// @version      0.1.1
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

    let colorScheme = {
        background: '#2d2d2d',
        text: '#ffffff',
        border: '#555555',
        buttonBg: '#444444',
        buttonText: '#ffffff',
        buttonHover: '#555555',
        inputBg: '#3d3d3d',
        inputBorder: '#666666',
        shadow: 'rgba(0,0,0,0.3)'
    };

    const container = document.createElement('div');
    container.style.position = 'fixed';
    container.style.bottom = '20px';
    container.style.left = '20px';
    container.style.padding = minimized ? '10px' : '15px';
    container.style.borderRadius = '8px';
    container.style.zIndex = '9999';
    container.style.fontFamily = 'Arial, sans-serif';
    container.style.fontSize = '14px';
    container.style.lineHeight = '1.5';
    container.style.width = minimized ? 'auto' : '220px';

    const titleDiv = document.createElement('div');
    titleDiv.textContent = '下载设置';
    titleDiv.style.fontWeight = 'bold';
    titleDiv.style.marginBottom = '15px';
    titleDiv.style.fontSize = '16px';
    titleDiv.style.textAlign = 'left';
    container.appendChild(titleDiv);

    const formContainer = document.createElement('div');
    formContainer.style.display = 'flex';
    formContainer.style.flexDirection = 'column';
    formContainer.style.gap = '12px';

    const originalFileDiv = document.createElement('div');
    formContainer.style.display = minimized ? 'none' : 'flex';
    originalFileDiv.style.alignItems = 'center';
    originalFileDiv.style.textAlign = 'left';

    const originalFileLabel = document.createElement('label');
    originalFileLabel.textContent = '原始档: ';
    originalFileLabel.style.marginRight = '10px';
    originalFileLabel.style.minWidth = '70px';

    const originalFileCheckbox = document.createElement('input');
    originalFileCheckbox.type = 'checkbox';
    originalFileCheckbox.checked = isOriginal;
    originalFileCheckbox.style.transform = 'scale(1.2)';
    originalFileCheckbox.style.cursor = 'pointer';
    originalFileCheckbox.addEventListener('change', function () {
        isOriginal = this.checked;
        GM_setValue('isOriginal', isOriginal);
    });

    originalFileDiv.appendChild(originalFileLabel);
    originalFileDiv.appendChild(originalFileCheckbox);
    formContainer.appendChild(originalFileDiv);

    const themeDiv = document.createElement('div');
    titleDiv.style.display = minimized ? 'none' : 'block';
    formContainer.appendChild(themeDiv);

    const backendUrlDiv = document.createElement('div');
    backendUrlDiv.style.display = 'flex';
    backendUrlDiv.style.flexDirection = 'column';
    backendUrlDiv.style.alignItems = 'flex-start';
    backendUrlDiv.style.textAlign = 'left';

    const backendUrlLabel = document.createElement('label');
    backendUrlLabel.textContent = '后端 URL:';
    backendUrlLabel.style.marginBottom = '5px';

    const backendUrlInput = document.createElement('input');
    backendUrlInput.type = 'text';
    backendUrlInput.value = backendUrl;
    backendUrlInput.style.width = '100%';
    backendUrlInput.style.padding = '8px';
    backendUrlInput.style.boxSizing = 'border-box';
    backendUrlInput.style.borderRadius = '4px';
    backendUrlInput.addEventListener('change', function () {
        backendUrl = this.value;
        GM_setValue('backendUrl', backendUrl);
    });

    backendUrlDiv.appendChild(backendUrlLabel);
    backendUrlDiv.appendChild(backendUrlInput);
    formContainer.appendChild(backendUrlDiv);

    const downloadButton = document.createElement('button');
    downloadButton.textContent = '下载';
    downloadButton.style.padding = '8px 12px';
    downloadButton.style.marginTop = '10px';
    downloadButton.style.cursor = 'pointer';
    downloadButton.style.borderRadius = '4px';
    downloadButton.style.fontWeight = 'bold';
    downloadButton.style.width = '100%';
    downloadButton.style.border = 'none';
    downloadButton.addEventListener('click', function () {
        sendDownloadRequest();
    });

    downloadButton.addEventListener('mouseover', function () {
        this.style.backgroundColor = colorScheme.buttonHover;
    });
    downloadButton.addEventListener('mouseout', function () {
        this.style.backgroundColor = colorScheme.buttonBg;
    });

    formContainer.appendChild(downloadButton);
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
            },
            onerror: function (error) {
                showNotification('发送下载请求时出错!', 'error');
            }
        });
    }

    function showNotification(message, type) {
        const notification = document.createElement('div');
        notification.textContent = message;
        notification.style.position = 'fixed';
        notification.style.bottom = '20px';
        notification.style.left = '20px';
        notification.style.padding = '10px 15px';
        notification.style.borderRadius = '4px';
        notification.style.zIndex = '10000';
        notification.style.color = '#fff';
        notification.style.backgroundColor = type === 'success' ? '#4CAF50' : '#F44336';
        notification.style.boxShadow = '0 2px 5px rgba(0,0,0,0.2)';
        notification.style.opacity = '0';
        notification.style.transition = 'opacity 0.3s ease';

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
    toggleButton.textContent = minimized ? '+' : '-';
    toggleButton.style.position = 'absolute';
    toggleButton.style.top = '10px';
    toggleButton.style.right = '10px';
    toggleButton.style.width = '20px';
    toggleButton.style.height = '20px';
    toggleButton.style.cursor = 'pointer';
    toggleButton.style.border = 'none';
    toggleButton.style.borderRadius = '50%';
    toggleButton.style.fontSize = '14px';
    toggleButton.style.display = 'flex';
    toggleButton.style.alignItems = 'center';
    toggleButton.style.justifyContent = 'center';
    toggleButton.style.padding = '0';

    toggleButton.addEventListener('click', function () {
        minimized = !minimized;
        GM_setValue('minimized', minimized);

        if (minimized) {
            toggleButton.textContent = '+';
            formContainer.style.display = 'none';
            titleDiv.style.display = 'none';
            container.style.padding = '10px';
            container.style.width = 'auto';
        } else {
            toggleButton.textContent = '-';
            formContainer.style.display = 'flex';
            titleDiv.style.display = 'block';
            container.style.padding = '15px';
            container.style.width = '220px';
        }
    });

    container.appendChild(toggleButton);

    function updateStyles() {
        container.style.backgroundColor = colorScheme.background;
        container.style.color = colorScheme.text;
        container.style.border = `1px solid ${colorScheme.border}`;
        container.style.boxShadow = `0 0 10px ${colorScheme.shadow}`;

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
})();