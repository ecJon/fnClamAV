const sharp = require('sharp');
const fs = require('fs');

const sourceImage = '/home/test/code/example/clamav-trademark.png';
const outputDir = '/home/test/code/example/app/ui/images';

// 定义需要生成的尺寸
const sizes = [
    { name: 'icon-64.png', size: 64 },
    { name: 'icon-256.png', size: 256 }
];

async function resizeIcons() {
    try {
        for (const { name, size } of sizes) {
            const outputPath = `${outputDir}/${name}`;
            console.log(`Resizing to ${name} (${size}x${size})...`);

            await sharp(sourceImage)
                .resize(size, size, {
                    fit: 'contain',
                    background: { r: 0, g: 0, b: 0, alpha: 0 }
                })
                .toFile(outputPath);

            console.log(`✓ Created ${name}`);
        }
        console.log('\n✅ All icons resized successfully!');
    } catch (error) {
        console.error('Error:', error);
        process.exit(1);
    }
}

resizeIcons();
