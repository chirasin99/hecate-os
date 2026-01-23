# HecateOS Repository Infrastructure

## Creating Your Own APT Repository

### Why Have Your Own Repository?

1. **Control**: Complete control over package versions and updates
2. **Custom Packages**: Distribute HecateOS-specific tools and configurations
3. **Faster Updates**: Push security patches immediately without waiting for upstream
4. **Optimized Binaries**: Compile packages with specific optimizations for target hardware
5. **Private Packages**: Proprietary or experimental packages not suitable for public repos

### Repository Structure

```
repo.hecate-os.io/
├── ubuntu/
│   ├── dists/
│   │   └── noble/
│   │       ├── Release
│   │       ├── Release.gpg
│   │       ├── InRelease
│   │       └── main/
│   │           ├── binary-amd64/
│   │           │   ├── Packages
│   │           │   ├── Packages.gz
│   │           │   └── Release
│   │           └── source/
│   └── pool/
│       └── main/
│           └── h/
│               └── hecate-cli/
│                   ├── hecate-cli_0.1.0_amd64.deb
│                   └── hecate-cli_0.1.0.dsc
└── KEY.gpg
```

### Setting Up Your Repository

#### 1. Create GPG Key for Signing

```bash
# Generate GPG key
gpg --full-generate-key

# Export public key
gpg --armor --export your-email@hecate-os.io > KEY.gpg

# Export private key (keep secure!)
gpg --armor --export-secret-keys your-email@hecate-os.io > private.key
```

#### 2. Build Your Packages

```bash
# Example: Creating hecate-cli package
mkdir -p hecate-cli/DEBIAN
cat > hecate-cli/DEBIAN/control << EOF
Package: hecate-cli
Version: 0.1.0
Architecture: amd64
Maintainer: HecateOS Team <team@hecate-os.io>
Description: HecateOS Command Line Interface
 Main CLI tool for HecateOS system management
Depends: python3 (>= 3.12)
Priority: optional
Section: admin
EOF

# Copy files
cp -r /usr/local/bin/hecate* hecate-cli/usr/local/bin/

# Build package
dpkg-deb --build hecate-cli
```

#### 3. Create Repository Structure

```bash
# Install repository tools
apt install reprepro apt-utils dpkg-dev

# Create repository configuration
mkdir -p repo/conf
cat > repo/conf/distributions << EOF
Origin: HecateOS
Label: HecateOS
Codename: noble
Architectures: amd64 source
Components: main
Description: HecateOS Official Repository
SignWith: your-key-id
EOF

# Add packages
reprepro -b ./repo includedeb noble hecate-cli_0.1.0_amd64.deb
```

#### 4. Host Your Repository

**Option A: GitHub Pages (Free)**
```bash
# Push to GitHub Pages branch
git checkout -b gh-pages
git add .
git commit -m "Update repository"
git push origin gh-pages
```

**Option B: S3 + CloudFront**
```bash
aws s3 sync ./repo s3://repo.hecate-os.io --delete
aws cloudfront create-invalidation --distribution-id ABCD1234 --paths "/*"
```

**Option C: Dedicated Server (nginx)**
```nginx
server {
    listen 443 ssl http2;
    server_name repo.hecate-os.io;
    
    root /var/www/repo;
    autoindex on;
    
    ssl_certificate /etc/ssl/certs/hecate.crt;
    ssl_certificate_key /etc/ssl/private/hecate.key;
    
    location / {
        try_files $uri $uri/ =404;
    }
}
```

### CI/CD Pipeline for Automatic Updates

```yaml
# .github/workflows/build-packages.yml
name: Build and Deploy Packages

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      
      - name: Build packages
        run: |
          ./scripts/build-all-packages.sh
      
      - name: Sign and publish
        env:
          GPG_KEY: ${{ secrets.GPG_PRIVATE_KEY }}
        run: |
          echo "$GPG_KEY" | gpg --import
          reprepro -b ./repo includedeb noble *.deb
      
      - name: Deploy to S3
        run: |
          aws s3 sync ./repo s3://repo.hecate-os.io --delete
```

### Package Priorities for HecateOS

#### High Priority Packages to Maintain:
1. **hecate-cli**: Main system management tool
2. **hecate-drivers**: GPU driver installer with latest versions
3. **hecate-kernel**: Custom optimized kernel builds
4. **hecate-ml-stack**: Pre-configured ML/AI packages
5. **hecate-monitoring**: System monitoring and telemetry

#### Version Policy:
- **Stable**: Well-tested, production-ready packages
- **Testing**: Beta packages for community testing
- **Experimental**: Bleeding-edge packages with latest features

### Security Considerations

1. **Sign all packages** with GPG
2. **Use HTTPS** for repository access
3. **Regular security audits** of hosted packages
4. **Automated vulnerability scanning** in CI/CD
5. **Reproducible builds** for transparency

### Cost Estimates

- **GitHub Pages**: Free (up to 100GB/month bandwidth)
- **S3 + CloudFront**: ~$50-200/month depending on traffic
- **VPS (Hetzner/OVH)**: ~$20-50/month
- **CDN (Bunny.net)**: ~$10-30/month

### Example Client Configuration

Users add your repository with:

```bash
# Add repository key
curl -fsSL https://repo.hecate-os.io/KEY.gpg | sudo apt-key add -

# Add repository
echo "deb https://repo.hecate-os.io/ubuntu noble main" | \
  sudo tee /etc/apt/sources.list.d/hecate.list

# Update and install
sudo apt update
sudo apt install hecate-cli
```

### Monitoring Your Repository

- Use **analytics** to track package downloads
- Monitor **bandwidth usage** to control costs
- Track **error rates** for failed downloads
- Set up **alerts** for repository health

This infrastructure gives HecateOS complete independence and control over software distribution!