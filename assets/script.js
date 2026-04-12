const API = {
  brightness: (val) => `/api/settings/display/brightness/${val}`,
  power: (val = 'toggle') => `/api/settings/power/state/${val}`,
  display: (val = 'toggle') => `/api/settings/display/state/${val}`,
  micVolume: (val) => `/api/settings/mic/volume/${val}`,
  micMute: (val = 'toggle') => `/api/settings/mic/mute/${val}`,
  speakerVolume: (val) => `/api/settings/speaker/volume/${val}`,
  speakerMute: (val = 'toggle') => `/api/settings/speaker/mute/${val}`,
  record: (val = 'start') => `/api/voice/state/${val}`,
  update: '/api/update',
  media: (action) => `/api/media/${action}`
};

async function callApi(url, method = 'GET') {
  try {
    const res = await fetch(url, { method });
    const text = await res.text();
    console.log(`API ${url} -> ${text}`);
    return text;
  } catch (err) { console.error(`API error ${url}`, err); }
}

document.addEventListener('DOMContentLoaded', () => {
  const brightnessSlider = document.getElementById('brightnessSlider');
  const brightnessVal = document.getElementById('brightnessVal');
  if (brightnessSlider) {
    brightnessSlider.addEventListener('input', (e) => {
      let v = e.target.value;
      brightnessVal.innerText = v + '%';
      callApi(API.brightness(v));
    });
  }

  const togglePowerBtn = document.getElementById('togglePowerBtn');
  if (togglePowerBtn) togglePowerBtn.addEventListener('click', () => callApi(API.power('toggle')));

  const displayOnOffBtn = document.getElementById('displayOnOffBtn');
  if (displayOnOffBtn) displayOnOffBtn.addEventListener('click', () => callApi(API.display('toggle')));

  const screensaverBtn = document.getElementById('screensaverBtn');
  if (screensaverBtn) screensaverBtn.addEventListener('click', () => console.log('Screensaver triggered'));

  const micSlider = document.getElementById('micSlider');
  const micVolVal = document.getElementById('micVolVal');
  if (micSlider && micVolVal) {
    micSlider.addEventListener('input', (e) => {
      let v = e.target.value;
      micVolVal.innerText = v + '%';
      callApi(API.micVolume(v));
    });
  }

  const micMuteBtn = document.getElementById('micMuteBtn');
  if (micMuteBtn) micMuteBtn.addEventListener('click', () => callApi(API.micMute('toggle')));

  const speakerSlider = document.getElementById('speakerSlider');
  const speakerVolVal = document.getElementById('speakerVolVal');
  if (speakerSlider && speakerVolVal) {
    speakerSlider.addEventListener('input', (e) => {
      let v = e.target.value;
      speakerVolVal.innerText = v + '%';
      callApi(API.speakerVolume(v));
    });
  }

  const speakerMuteBtn = document.getElementById('speakerMuteBtn');
  if (speakerMuteBtn) speakerMuteBtn.addEventListener('click', () => callApi(API.speakerMute('toggle')));

  const recordBtn = document.getElementById('recordBtn');
  if (recordBtn) recordBtn.addEventListener('click', () => callApi(API.record('start')));

  const updateBtn = document.getElementById('updateBtn');
  if (updateBtn) updateBtn.addEventListener('click', () => callApi(API.update));

  const mediaPrev = document.getElementById('mediaPrev');
  if (mediaPrev) mediaPrev.addEventListener('click', () => callApi(API.media('prev')));

  const mediaPlayPause = document.getElementById('mediaPlayPause');
  if (mediaPlayPause) mediaPlayPause.addEventListener('click', () => callApi(API.media('playpause')));

  const mediaStop = document.getElementById('mediaStop');
  if (mediaStop) mediaStop.addEventListener('click', () => callApi(API.media('stop')));

  const mediaNext = document.getElementById('mediaNext');
  if (mediaNext) mediaNext.addEventListener('click', () => callApi(API.media('next')));
});

function updateTime() {
  const liveTime = document.getElementById('liveTime');
  if (liveTime) liveTime.innerText = new Date().toLocaleTimeString();
}
setInterval(updateTime, 1000);
updateTime();

function randomizeTelemetry() {
  const battVoltage = document.getElementById('battVoltage');
  if (battVoltage) battVoltage.innerText = (3.7 + Math.random() * 0.5).toFixed(2) + ' V';
  const battPercent = document.getElementById('battPercent');
  if (battPercent) battPercent.innerText = Math.floor(40 + Math.random() * 60) + ' %';
  const temperature = document.getElementById('temperature');
  if (temperature) temperature.innerText = (18 + Math.random() * 12).toFixed(1) + ' °C';
  const humidity = document.getElementById('humidity');
  if (humidity) humidity.innerText = Math.floor(30 + Math.random() * 45) + ' %';
  const rssi = document.getElementById('rssi');
  if (rssi) rssi.innerText = -Math.floor(30 + Math.random() * 55) + ' dBm';
  const occupancy = document.getElementById('occupancy');
  if (occupancy) occupancy.innerHTML = Math.random() > 0.8 ? '👤 DETECTED' : '🌿 CLEAR';
  const irStatus = document.getElementById('irStatus');
  if (irStatus) {
    if (Math.random() < 0.2) irStatus.innerText = '0x' + Math.floor(Math.random()*65535).toString(16);
    else irStatus.innerText = '—';
  }
}
setInterval(randomizeTelemetry, 7000);
randomizeTelemetry();
