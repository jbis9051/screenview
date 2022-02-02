export default function formatID(id: string) {
    let format = id.substring(0, 13).replaceAll(/[^0-9]/g, '');
    if (format.length > 9) {
        format = `${format[0]} ${format
            .substring(1)
            .replaceAll(/([0-9]{3})/g, '$1 ')}`;
    } else {
        format = format.replaceAll(/([0-9]{3})/g, '$1 ');
    }
    return format.trim();
}
